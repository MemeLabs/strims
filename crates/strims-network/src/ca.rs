// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The network CA: renews members' certificates and finds the ones it
//! issued, for a network whose server config (its root key) this device
//! holds. Its records are the store's certificate logs, network peers and
//! alias reservations.
//!
//! # Renewal
//!
//! A renewal presents the member's current certificate and a request
//! signed by the same key. The CA issues a certificate under a fresh root
//! when:
//!
//! - the current certificate verifies in full, expiry included, and is on
//!   this network;
//! - the request is for the certificate's key, signed by it, and asks for
//!   no more than peer and sign usage;
//! - the member isn't banned.
//!
//! A key the CA hasn't seen joins as a new peer. Its certificate must have
//! been issued under an invitation: four certificates, the third the
//! inviter's membership. The inviter must be a peer that isn't banned and
//! has invitations left, and spends one. A peer's alias is its request's
//! subject: it takes the alias's reservation, which another key holds
//! until [`ALIAS_RESERVATION_COOLDOWN`] after giving it up, and a peer
//! changes it at most once per [`ALIAS_CHANGE_COOLDOWN`].

use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_crypto::{
    DEFAULT_CERT_TTL, certificate_chain, certificate_network_key,
    sign_certificate_request_with_network, verify_certificate, verify_certificate_request,
};
use strims_dao::schema::network::{
    AliasReservations, CertificateLogs, NetworkPeers, RESERVED_FOREVER, network_key,
    network_scoped_key, new_alias_reservation, new_certificate_log, new_network_peer,
};
use strims_dao::{ReplicatedStore, Table, WriteTx};
use strims_kv::Kv;
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::network::v1::errors::ErrorCode;
use strims_proto::network::v1::{Network, Peer};
use strims_proto::service::Error as RpcError;
use strims_proto::types::certificate::ParentOneof;
use strims_proto::types::{Certificate, CertificateRequest, KeyUsage};

/// The salt the CA's address is published under, with the network's key.
pub const CA_SALT: &[u8] = b"ca";

/// How long a peer keeps its alias before it may change it again.
pub const ALIAS_CHANGE_COOLDOWN: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// How long an alias a peer gave up stays reserved.
pub const ALIAS_RESERVATION_COOLDOWN: Duration = Duration::from_secs(180 * 24 * 60 * 60);

/// The longest subject the CA issues a certificate for.
pub const MAX_SUBJECT_LEN: usize = 256;

/// The usages a member's certificate may have.
pub(crate) const MEMBER_USAGE: i32 = KeyUsage::Peer as i32 | KeyUsage::Sign as i32;

/// The longest chain the CA reads.
const MAX_CHAIN_DEPTH: usize = 8;

/// Why the CA refused.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CaError {
    #[error("{0}")]
    Invalid(&'static str),
    #[error("network has no server config")]
    NotCa,
    #[error("alias in use")]
    AliasInUse,
    #[error("alias was changed too recently")]
    AliasCooldown,
    #[error("invitation quota exceeded")]
    QuotaExceeded,
    #[error("inviter banned")]
    InviterBanned,
    #[error("peer banned")]
    Banned,
    #[error("not found")]
    NotFound,
    #[error("too many renewals")]
    RateLimited,
    #[error(transparent)]
    Store(#[from] strims_dao::Error),
}

impl CaError {
    pub(crate) fn code(&self) -> ErrorCode {
        match self {
            CaError::AliasInUse => ErrorCode::AliasInUse,
            CaError::AliasCooldown => ErrorCode::AliasChangeCooldownViolatied,
            CaError::QuotaExceeded => ErrorCode::InvitationQuotaExceeded,
            CaError::InviterBanned => ErrorCode::InviterBanned,
            _ => ErrorCode::Unknown,
        }
    }

    pub(crate) fn to_rpc(&self) -> RpcError {
        RpcError::new(self.code(), self.to_string())
    }
}

/// Issues a renewal of `cert` for `csr` on `network`, whose server config
/// this device holds, and records it. See the [module docs](self).
pub(crate) async fn renew<K: Kv>(
    store: &ReplicatedStore<K>,
    network: &Network,
    cert: &Certificate,
    csr: &CertificateRequest,
    now: Time,
    rng: &mut DetRng,
) -> Result<Certificate, CaError> {
    let config = network.server_config.as_ref().ok_or(CaError::NotCa)?;
    let key = config.key.as_ref().ok_or(CaError::NotCa)?;
    let chain = certificate_chain(cert);
    if chain.len() > MAX_CHAIN_DEPTH {
        return Err(CaError::Invalid("certificate chain too long"));
    }
    verify_certificate(cert, now).map_err(|_| CaError::Invalid("invalid certificate"))?;
    if certificate_network_key(cert) != network_key(network) {
        return Err(CaError::Invalid("certificate for another network"));
    }
    if cert.key != csr.key {
        return Err(CaError::Invalid(
            "certificate key does not match the request key",
        ));
    }
    verify_certificate_request(csr, MEMBER_USAGE)
        .map_err(|_| CaError::Invalid("invalid certificate request"))?;
    if csr.subject.len() > MAX_SUBJECT_LEN {
        return Err(CaError::Invalid("subject too long"));
    }
    let inviter_key = chain.get(2).map(|c| c.key.clone());
    let issued =
        sign_certificate_request_with_network(csr, key, &config.name, DEFAULT_CERT_TTL, now, rng)
            .map_err(|_| CaError::Invalid("signing failed"))?;

    let mut tx = store.write(now).await?;
    let id = network.id;
    let existing = NetworkPeers::index_get(
        &mut tx,
        &NetworkPeers::BY_PUBLIC_KEY,
        &network_scoped_key(id, &issued.key),
    )
    .await?;
    match existing {
        Some(peer) => update_peer(&mut tx, peer, &issued, now).await?,
        None => {
            let inviter_key = inviter_key
                .filter(|_| chain.len() == 4)
                .ok_or(CaError::Invalid("malformed invite certificate chain"))?;
            insert_peer(&mut tx, id, &issued, &inviter_key, now).await?;
        }
    }
    let log_id = tx.generate_id().await?;
    CertificateLogs::insert(&mut tx, new_certificate_log(log_id, id, &issued)).await?;
    tx.commit().await?;
    Ok(issued)
}

async fn update_peer<K: Kv>(
    tx: &mut WriteTx<K>,
    mut peer: Peer,
    cert: &Certificate,
    now: Time,
) -> Result<(), CaError> {
    if peer.is_banned {
        return Err(CaError::Banned);
    }
    if peer.alias == cert.subject {
        return Ok(());
    }
    let changed = Time::from_unix(peer.alias_changed_at, 0);
    if !now.after(changed + ALIAS_CHANGE_COOLDOWN) {
        return Err(CaError::AliasCooldown);
    }
    reserve_alias(tx, peer.network_id, &cert.subject, &cert.key, now).await?;
    release_alias(tx, peer.network_id, &peer.alias, now).await?;
    peer.alias = cert.subject.clone();
    peer.alias_changed_at = now.unix();
    NetworkPeers::update(tx, peer).await?;
    Ok(())
}

async fn insert_peer<K: Kv>(
    tx: &mut WriteTx<K>,
    network_id: u64,
    cert: &Certificate,
    inviter_key: &[u8],
    now: Time,
) -> Result<(), CaError> {
    reserve_alias(tx, network_id, &cert.subject, &cert.key, now).await?;
    let mut inviter = NetworkPeers::index_get(
        tx,
        &NetworkPeers::BY_PUBLIC_KEY,
        &network_scoped_key(network_id, inviter_key),
    )
    .await?
    .ok_or(CaError::Invalid("inviter not found"))?;
    if inviter.is_banned {
        return Err(CaError::InviterBanned);
    }
    if inviter.invite_quota == 0 {
        return Err(CaError::QuotaExceeded);
    }
    inviter.invite_quota -= 1;
    let inviter_id = inviter.id;
    NetworkPeers::update(tx, inviter).await?;
    let id = tx.generate_id().await?;
    let peer = new_network_peer(id, network_id, &cert.key, &cert.subject, inviter_id, now);
    NetworkPeers::insert(tx, peer).await?;
    Ok(())
}

/// Gives `alias` on the network to `peer_key`: one it already holds, a
/// free one, or one whose reservation has lapsed.
pub(crate) async fn reserve_alias<K: Kv>(
    tx: &mut WriteTx<K>,
    network_id: u64,
    alias: &str,
    peer_key: &[u8],
    now: Time,
) -> Result<(), CaError> {
    let found = AliasReservations::index_get(
        tx,
        &AliasReservations::BY_ALIAS,
        &network_scoped_key(network_id, alias.as_bytes()),
    )
    .await?;
    let Some(mut r) = found else {
        let id = tx.generate_id().await?;
        let r = new_alias_reservation(id, network_id, alias, peer_key);
        AliasReservations::insert(tx, r).await?;
        return Ok(());
    };
    if r.peer_key == peer_key {
        return Ok(());
    }
    if now.unix() <= r.reserved_until {
        return Err(CaError::AliasInUse);
    }
    r.peer_key = peer_key.to_vec();
    r.reserved_until = RESERVED_FOREVER;
    AliasReservations::update(tx, r).await?;
    Ok(())
}

/// Frees `alias` on the network, keeping it reserved for
/// [`ALIAS_RESERVATION_COOLDOWN`].
pub(crate) async fn release_alias<K: Kv>(
    tx: &mut WriteTx<K>,
    network_id: u64,
    alias: &str,
    now: Time,
) -> Result<(), CaError> {
    let found = AliasReservations::index_get(
        tx,
        &AliasReservations::BY_ALIAS,
        &network_scoped_key(network_id, alias.as_bytes()),
    )
    .await?;
    if let Some(mut r) = found {
        r.peer_key.clear();
        r.reserved_until = (now + ALIAS_RESERVATION_COOLDOWN).unix();
        AliasReservations::update(tx, r).await?;
    }
    Ok(())
}

/// The certificate the CA of network `network_id` issued that matches
/// `query`: the latest for a subject or key. With `full_chain`, its
/// parents are filled in from their logs.
pub(crate) async fn find<K: Kv>(
    store: &ReplicatedStore<K>,
    network_id: u64,
    query: Option<&Query>,
    full_chain: bool,
) -> Result<Certificate, CaError> {
    let mut tx = store.read().await?;
    let scoped = |b: &[u8]| network_scoped_key(network_id, b);
    let (ix, key) = match query.ok_or(CaError::Invalid("find needs a subject, serial or key"))? {
        Query::Subject(s) => (&CertificateLogs::BY_SUBJECT, scoped(s.as_bytes())),
        Query::SerialNumber(s) => (&CertificateLogs::BY_SERIAL_NUMBER, scoped(s)),
        Query::Key(k) => (&CertificateLogs::BY_KEY, scoped(k)),
    };
    let log = CertificateLogs::index_get(&mut tx, ix, &key)
        .await?
        .ok_or(CaError::NotFound)?;
    let cert = log.certificate.ok_or(CaError::NotFound)?;
    if !full_chain {
        return Ok(cert);
    }
    let mut chain = vec![cert];
    while let Some(Certificate {
        parent_oneof: Some(ParentOneof::ParentSerialNumber(serial)),
        ..
    }) = chain.last()
    {
        if chain.len() >= MAX_CHAIN_DEPTH {
            return Err(CaError::Invalid("certificate chain too long"));
        }
        let parent = CertificateLogs::index_get(
            &mut tx,
            &CertificateLogs::BY_SERIAL_NUMBER,
            &scoped(serial),
        )
        .await?
        .and_then(|l| l.certificate)
        .ok_or(CaError::NotFound)?;
        chain.push(parent);
    }
    let mut cert = chain.pop().ok_or(CaError::NotFound)?;
    while let Some(mut child) = chain.pop() {
        child.parent_oneof = Some(ParentOneof::Parent(Box::new(cert)));
        cert = child;
    }
    Ok(cert)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

    use std::future::Future;

    use futures_util::FutureExt;
    use strims_base::rng::det_rng;
    use strims_crypto::{generate_key, new_certificate_request};
    use strims_dao::schema::network::Networks;
    use strims_dao::schema::profile::Profile as ProfileRecord;
    use strims_dao::seal::StorageKey;
    use strims_dao::{ProfileStore, Singleton};
    use strims_kv::MemoryKv;
    use strims_proto::profile::v1::Profile;
    use strims_proto::types::Key;

    use super::*;
    use crate::membership::{network_from_invitation_v0, new_invitation_v0, new_network};

    const T0: Time = Time::from_unix_millis(1_700_000_000_000);

    fn block<T>(f: impl Future<Output = T>) -> T {
        f.now_or_never().expect("memory store operations are ready")
    }

    fn profile(n: u8) -> Profile {
        Profile {
            id: u64::from(n),
            name: format!("p{n}"),
            key: Some(generate_key(&mut det_rng(u64::from(n)))),
            device_id: 7,
            ..Default::default()
        }
    }

    struct Ca {
        store: ReplicatedStore<MemoryKv>,
        network: Network,
        owner: Profile,
        rng: DetRng,
    }

    impl Ca {
        fn new() -> Ca {
            let owner = profile(1);
            let store = block(async {
                let store = ProfileStore::create(
                    MemoryKv::new(),
                    1,
                    StorageKey::from_bytes([1; 32]),
                    det_rng(1),
                    T0,
                )
                .await
                .unwrap();
                let mut tx = store.write(T0).await.unwrap();
                ProfileRecord::set(&mut tx, owner.clone()).await.unwrap();
                tx.commit().await.unwrap();
                ReplicatedStore::new(store).await.unwrap()
            });
            let mut rng = det_rng(2);
            let network = new_network(10, "net", None, &owner, "", T0, &mut rng).unwrap();
            block(async {
                let mut tx = store.write(T0).await.unwrap();
                Networks::insert(&mut tx, network.clone()).await.unwrap();
                tx.commit().await.unwrap();
            });
            Ca {
                store,
                network,
                owner,
                rng,
            }
        }

        fn owner_cert(&self) -> Certificate {
            self.network.certificate.clone().unwrap()
        }

        /// `who`'s provisional certificate under an invitation from the
        /// holder of `inviter` and `inviter_cert`.
        fn invited(
            &mut self,
            inviter: &Profile,
            inviter_cert: &Certificate,
            who: &Profile,
        ) -> Certificate {
            let inv = new_invitation_v0(inviter, inviter_cert, T0, &mut self.rng).unwrap();
            let n = network_from_invitation_v0(1, &inv, who, "", T0, &mut self.rng).unwrap();
            n.certificate.unwrap()
        }

        fn renew(
            &mut self,
            cert: &Certificate,
            key: &Key,
            subject: &str,
            at: Time,
        ) -> Result<Certificate, CaError> {
            let csr = new_certificate_request(key, MEMBER_USAGE, subject).unwrap();
            block(renew(
                &self.store,
                &self.network,
                cert,
                &csr,
                at,
                &mut self.rng,
            ))
        }

        fn peer(&self, key: &[u8]) -> Peer {
            block(async {
                let mut tx = self.store.read().await.unwrap();
                NetworkPeers::index_get(
                    &mut tx,
                    &NetworkPeers::BY_PUBLIC_KEY,
                    &network_scoped_key(10, key),
                )
                .await
                .unwrap()
                .unwrap()
            })
        }

        fn set_peer(&self, p: Peer) {
            block(async {
                let mut tx = self.store.write(T0).await.unwrap();
                NetworkPeers::update(&mut tx, p).await.unwrap();
                tx.commit().await.unwrap();
            });
        }
    }

    fn key_of(p: &Profile) -> &Key {
        p.key.as_ref().unwrap()
    }

    #[test]
    fn an_invitee_joins_under_its_inviter() {
        let mut ca = Ca::new();
        let (owner, alice) = (ca.owner.clone(), profile(2));
        let provisional = ca.invited(&owner, &ca.owner_cert(), &alice);
        let issued = ca.renew(&provisional, key_of(&alice), "alice", T0).unwrap();
        assert!(strims_crypto::membership_trusted(&issued, T0));
        assert_eq!(issued.subject, "alice");
        let peer = ca.peer(&key_of(&alice).public);
        let inviter = ca.peer(&key_of(&owner).public);
        assert_eq!(peer.inviter_peer_id, inviter.id);
        assert_eq!((peer.alias.as_str(), inviter.invite_quota), ("alice", 2));

        let found = block(find(
            &ca.store,
            10,
            Some(&Query::Subject("alice".into())),
            false,
        ))
        .unwrap();
        assert_eq!(found.serial_number, issued.serial_number);
        let by_key = block(find(
            &ca.store,
            10,
            Some(&Query::Key(issued.key.clone())),
            false,
        ))
        .unwrap();
        assert_eq!(by_key.serial_number, issued.serial_number);
        // a renewal's root is new and unlogged, so its chain can't be
        // rebuilt; the owner's first chain is logged whole
        assert!(matches!(
            block(find(
                &ca.store,
                10,
                Some(&Query::Subject("alice".into())),
                true
            )),
            Err(CaError::NotFound)
        ));
        let owner_chain = block(find(
            &ca.store,
            10,
            Some(&Query::Subject("p1".into())),
            true,
        ))
        .unwrap();
        assert_eq!(owner_chain, ca.owner_cert());
    }

    #[test]
    fn invitations_need_an_inviter_with_quota_who_isnt_banned() {
        let mut ca = Ca::new();
        let owner = ca.owner.clone();
        let mut inviter = ca.peer(&key_of(&owner).public);
        inviter.invite_quota = 0;
        ca.set_peer(inviter.clone());
        let a = profile(2);
        let provisional = ca.invited(&owner, &ca.owner_cert(), &a);
        assert!(matches!(
            ca.renew(&provisional, key_of(&a), "a", T0),
            Err(CaError::QuotaExceeded)
        ));
        inviter.invite_quota = 1;
        inviter.is_banned = true;
        ca.set_peer(inviter);
        assert!(matches!(
            ca.renew(&provisional, key_of(&a), "a", T0),
            Err(CaError::InviterBanned)
        ));
    }

    #[test]
    fn aliases_are_reserved_and_changes_cool_down() {
        let mut ca = Ca::new();
        let owner = ca.owner.clone();
        let (a, b) = (profile(2), profile(3));
        let pa = ca.invited(&owner, &ca.owner_cert(), &a);
        let a_cert = ca.renew(&pa, key_of(&a), "name", T0).unwrap();
        let pb = ca.invited(&owner, &ca.owner_cert(), &b);
        assert!(matches!(
            ca.renew(&pb, key_of(&b), "name", T0),
            Err(CaError::AliasInUse)
        ));
        ca.renew(&pb, key_of(&b), "other", T0).unwrap();

        // a new member may rename once; then once per cooldown
        let soon = T0 + Duration::HOUR;
        let a_cert = ca.renew(&a_cert, key_of(&a), "renamed", soon).unwrap();
        assert!(matches!(
            ca.renew(&a_cert, key_of(&a), "again", soon + Duration::HOUR),
            Err(CaError::AliasCooldown)
        ));
        // the alias it gave up stays reserved for its cooldown
        let pc = ca.invited(&owner, &ca.owner_cert(), &profile(4));
        let r = ca.renew(&pc, key_of(&profile(4)), "name", soon + Duration::HOUR);
        assert!(matches!(r, Err(CaError::AliasInUse)), "{r:?}");
        let later = soon + ALIAS_CHANGE_COOLDOWN + Duration::HOUR;
        ca.renew(&a_cert, key_of(&a), "again", later).unwrap();
    }

    #[test]
    fn banned_peers_and_foreign_requests_are_refused() {
        let mut ca = Ca::new();
        let owner = ca.owner.clone();
        let a = profile(2);
        let pa = ca.invited(&owner, &ca.owner_cert(), &a);
        let a_cert = ca.renew(&pa, key_of(&a), "a", T0).unwrap();
        // a request for another key
        assert!(matches!(
            ca.renew(&a_cert, key_of(&profile(5)), "a", T0),
            Err(CaError::Invalid(_))
        ));
        let mut peer = ca.peer(&key_of(&a).public);
        peer.is_banned = true;
        ca.set_peer(peer);
        assert!(matches!(
            ca.renew(&a_cert, key_of(&a), "a", T0),
            Err(CaError::Banned)
        ));
        // a certificate of another network
        let other = new_network(11, "x", None, &a, "", T0, &mut ca.rng).unwrap();
        assert!(matches!(
            ca.renew(other.certificate.as_ref().unwrap(), key_of(&a), "a", T0),
            Err(CaError::Invalid(_))
        ));
    }
}
