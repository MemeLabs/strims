// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Networks the profile belongs to, its bootstrap clients and bootstrap
//! config, and the records of the networks whose CA it runs.
//!
//! The bootstrap config stays on the device: each device decides whether
//! peers may publish networks to it.
//!
//! A network's CA lives on the device that holds its server config, so
//! its records (certificate logs, network peers, alias reservations) don't
//! replicate. Inserting a network with a server config on this device
//! creates the CA's records for its owner and makes the owner the admin of
//! the network's directory ([`super::directory`]). Deleting a network
//! deletes its CA and directory records, the chat servers on it
//! ([`super::chat`]) and the notifications about it
//! ([`super::notification`]).

use futures_util::future::LocalBoxFuture;
use prost::Message;
use strims_base::time::Time;
use strims_crypto::{certificate_chain, certificate_network_key};
use strims_kv::Kv;
use strims_proto::network::v1::bootstrap::{self, BootstrapClient, bootstrap_client};
use strims_proto::network::v1::ca::CertificateLog;
use strims_proto::network::v1::directory::UserModeration;
use strims_proto::network::v1::{AliasReservation, Network, Peer, UiConfig};
use strims_proto::types::Certificate;
use strims_proto::types::certificate::ParentOneof;

use crate::error::Error;
use crate::events::Record;
use crate::key::Namespace;
use crate::schema::chat::ChatServers;
use crate::schema::directory::{DirectoryListingRecords, DirectoryUserRecords, new_user_record};
use crate::schema::notification::{delete_for_subject, network_subject};
use crate::store::WriteTx;
use crate::table::{
    ApplyConflict, Index, Replication, Singleton, Table, UniqueConflict, ref_key,
    resolve_last_written, upgrade_version,
};

/// The profile's networks. A network's server config (its CA) stays on
/// the device that holds it: other replicas receive networks without it,
/// and keep their own when they apply a change.
pub struct Networks;

impl Networks {
    /// Networks by their root certificate's key.
    pub const BY_KEY: Index<Network> = Index::unique(Namespace(4002), network_key);
}

/// A network's key: its root certificate's key.
pub fn network_key(n: &Network) -> Vec<u8> {
    n.certificate
        .as_ref()
        .map(|c| certificate_network_key(c).to_vec())
        .unwrap_or_default()
}

impl Table for Networks {
    type Record = Network;
    const NS: Namespace = Namespace(4001);
    const INDEXES: &'static [Index<Network>] = &[Networks::BY_KEY];
    const REPLICATION: Option<Replication<Network>> = Some(Replication {
        extract: |mut n| {
            n.server_config = None;
            n
        },
        merge: |mut next, local| {
            upgrade_version(&mut next, local);
            next.server_config = local.server_config.clone();
            next
        },
        resolve: resolve_network,
        ..Replication::DEFAULT
    });

    fn id(r: &Network) -> u64 {
        r.id
    }

    fn observe(r: &Network) -> Option<Record> {
        Some(Record::Network(r.clone()))
    }

    /// A network's record replaces another for the same network, taking
    /// over its newer fields, unless the other holds the network's server
    /// config.
    fn on_unique_conflict(
        _: &Index<Network>,
        m: &mut Network,
        existing: &Network,
    ) -> UniqueConflict {
        if existing.server_config.is_some() {
            return UniqueConflict::Reject;
        }
        fold_network(m, existing);
        UniqueConflict::Replace
    }

    /// Server configs don't replicate, so no replica can tell whether
    /// deleting another's record for a network would delete its CA.
    const ON_APPLY_CONFLICT: ApplyConflict = ApplyConflict::KeepLocal;

    /// A network with a server config written here for the first time
    /// gets its CA's records for the owner: a log of each certificate in
    /// the owner's chain, the owner as the network's first peer, and the
    /// owner's alias. The owner becomes its directory's admin and
    /// moderator.
    fn on_change<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        m: &'a mut Network,
        p: Option<&'a Network>,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            if p.is_some() || m.server_config.is_none() || tx.applying() {
                return Ok(());
            }
            let Some(cert) = m.certificate.as_ref() else {
                return Ok(());
            };
            for c in certificate_chain(cert) {
                let id = tx.generate_id().await?;
                CertificateLogs::insert(tx, new_certificate_log(id, m.id, c)).await?;
            }
            let id = tx.generate_id().await?;
            let now = tx.now();
            let peer = new_network_peer(id, m.id, &cert.key, &cert.subject, 0, now);
            NetworkPeers::insert(tx, peer).await?;
            let id = tx.generate_id().await?;
            let reservation = new_alias_reservation(id, m.id, &cert.subject, &cert.key);
            AliasReservations::insert(tx, reservation).await?;
            let id = tx.generate_id().await?;
            let admin = UserModeration {
                is_admin: Some(true),
                is_moderator: Some(true),
                ..Default::default()
            };
            DirectoryUserRecords::insert(tx, new_user_record(id, m.id, &cert.key, admin)).await?;
            Ok(())
        })
    }

    /// Deleting a network deletes its CA's records, its directory records,
    /// the chat servers on it and the notifications about it.
    fn on_delete<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        p: &'a Network,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            let key = network_key(p);
            if !key.is_empty() {
                // Chat servers are local to this device and few, so a scan
                // finds the network's.
                for server in ChatServers::get_all(tx).await? {
                    if server.network_key == key {
                        ChatServers::delete(tx, server.id).await?;
                    }
                }
            }
            let network = ref_key(p.id);
            for id in CertificateLogs::index_ids(tx, &CertificateLogs::BY_NETWORK, &network).await?
            {
                CertificateLogs::delete(tx, id).await?;
            }
            for id in NetworkPeers::index_ids(tx, &NetworkPeers::BY_NETWORK, &network).await? {
                NetworkPeers::delete(tx, id).await?;
            }
            for id in
                AliasReservations::index_ids(tx, &AliasReservations::BY_NETWORK, &network).await?
            {
                AliasReservations::delete(tx, id).await?;
            }
            for id in DirectoryListingRecords::index_ids(
                tx,
                &DirectoryListingRecords::BY_NETWORK,
                &network,
            )
            .await?
            {
                DirectoryListingRecords::delete(tx, id).await?;
            }
            for id in
                DirectoryUserRecords::index_ids(tx, &DirectoryUserRecords::BY_NETWORK, &network)
                    .await?
            {
                DirectoryUserRecords::delete(tx, id).await?;
            }
            delete_for_subject(tx, network_subject(p.id)).await
        })
    }
}

fn updated_at(n: &Network) -> i64 {
    n.version.as_ref().map_or(0, |v| v.updated_at)
}

fn not_after(n: &Network) -> u64 {
    n.certificate.as_ref().map_or(0, |c| c.not_after)
}

/// Folds `p` into `m`: the alias and renewal error written last, the
/// certificate that expires last, `p`'s server config, and both versions.
fn fold_network(m: &mut Network, p: &Network) {
    if updated_at(m) < updated_at(p) {
        m.alias = p.alias.clone();
        m.certificate_renewal_error = p.certificate_renewal_error;
    }
    if not_after(m) < not_after(p) {
        m.certificate = p.certificate.clone();
    }
    m.server_config = p.server_config.clone();
    upgrade_version(m, p);
}

/// Resolves concurrent writes to a network: the later record, with the
/// certificate that expires last of the two and the local server config.
fn resolve_network(next: Network, local: &Network) -> Network {
    let cert_key = |n: &Network| {
        (
            not_after(n),
            n.certificate.as_ref().map(Message::encode_to_vec),
        )
    };
    let certificate = if cert_key(local) > cert_key(&next) {
        local.certificate.clone()
    } else {
        next.certificate.clone()
    };
    let server_config = local.server_config.clone();
    let mut n = resolve_last_written(next, local);
    n.certificate = certificate;
    n.server_config = server_config;
    n
}

/// Bootstrap peers the profile's devices dial.
pub struct BootstrapClients;

impl BootstrapClients {
    /// Bootstrap clients by their websocket url. A new client replaces
    /// one with the same url.
    pub const BY_CLIENT_OPTIONS: Index<BootstrapClient> =
        Index::unique(Namespace(4014), client_options_key);
}

/// The key of a bootstrap client's options: its websocket url.
pub fn client_options_key(c: &BootstrapClient) -> Vec<u8> {
    match &c.client_options {
        Some(bootstrap_client::ClientOptions::WebsocketOptions(o)) => o.url.as_bytes().to_vec(),
        None => Vec::new(),
    }
}

impl Table for BootstrapClients {
    type Record = BootstrapClient;
    const NS: Namespace = Namespace(4007);
    const INDEXES: &'static [Index<BootstrapClient>] = &[BootstrapClients::BY_CLIENT_OPTIONS];
    const REPLICATION: Option<Replication<BootstrapClient>> = Some(Replication::DEFAULT);

    fn id(r: &BootstrapClient) -> u64 {
        r.id
    }

    fn observe(r: &BootstrapClient) -> Option<Record> {
        Some(Record::BootstrapClient(r.clone()))
    }

    fn on_unique_conflict(
        _: &Index<BootstrapClient>,
        _: &mut BootstrapClient,
        _: &BootstrapClient,
    ) -> UniqueConflict {
        UniqueConflict::Replace
    }
}

/// The key of a value within a network: the network id, big-endian,
/// then the value's bytes.
pub fn network_scoped_key(network_id: u64, value: &[u8]) -> Vec<u8> {
    let mut k = Vec::with_capacity(8 + value.len());
    k.extend_from_slice(&network_id.to_be_bytes());
    k.extend_from_slice(value);
    k
}

/// The certificates a network's CA issued, and the chain of its owner's
/// first certificate. A log's certificate names its parent by serial
/// number.
pub struct CertificateLogs;

impl CertificateLogs {
    /// Logs by network.
    pub const BY_NETWORK: Index<CertificateLog> =
        Index::new(Namespace(4004), |r| ref_key(r.network_id));
    /// Logs by (network, serial number); serial numbers are unique.
    pub const BY_SERIAL_NUMBER: Index<CertificateLog> = Index::unique(Namespace(4005), |r| {
        let serial = log_certificate(r).map_or(&[][..], |c| c.serial_number.as_slice());
        network_scoped_key(r.network_id, serial)
    });
    /// The latest log for each (network, subject).
    pub const BY_SUBJECT: Index<CertificateLog> = Index::unique(Namespace(4006), |r| {
        let subject = log_certificate(r).map_or("", |c| c.subject.as_str());
        network_scoped_key(r.network_id, subject.as_bytes())
    });
    /// The latest log for each (network, key).
    pub const BY_KEY: Index<CertificateLog> = Index::unique(Namespace(4013), |r| {
        let key = log_certificate(r).map_or(&[][..], |c| c.key.as_slice());
        network_scoped_key(r.network_id, key)
    });
}

fn log_certificate(r: &CertificateLog) -> Option<&Certificate> {
    r.certificate.as_ref()
}

impl Table for CertificateLogs {
    type Record = CertificateLog;
    const NS: Namespace = Namespace(4003);
    const INDEXES: &'static [Index<CertificateLog>] = &[
        CertificateLogs::BY_NETWORK,
        CertificateLogs::BY_SERIAL_NUMBER,
        CertificateLogs::BY_SUBJECT,
        CertificateLogs::BY_KEY,
    ];

    fn id(r: &CertificateLog) -> u64 {
        r.id
    }

    /// A newer log for a subject or key takes it over; serial numbers
    /// never repeat.
    fn on_unique_conflict(
        ix: &Index<CertificateLog>,
        _: &mut CertificateLog,
        _: &CertificateLog,
    ) -> UniqueConflict {
        if ix.ns == CertificateLogs::BY_SERIAL_NUMBER.ns {
            UniqueConflict::Reject
        } else {
            UniqueConflict::Steal
        }
    }
}

/// A log of `cert`, issued on network `network_id`, naming its parent by
/// serial number.
pub fn new_certificate_log(id: u64, network_id: u64, cert: &Certificate) -> CertificateLog {
    let mut cert = cert.clone();
    if let Some(ParentOneof::Parent(p)) = &cert.parent_oneof {
        cert.parent_oneof = Some(ParentOneof::ParentSerialNumber(p.serial_number.clone()));
    }
    CertificateLog {
        id,
        network_id,
        certificate: Some(cert),
    }
}

/// The members of networks whose CA this device runs.
pub struct NetworkPeers;

impl NetworkPeers {
    /// Peers by network.
    pub const BY_NETWORK: Index<Peer> = Index::new(Namespace(4010), |r| ref_key(r.network_id));
    /// Peers by (network, public key); at most one per key.
    pub const BY_PUBLIC_KEY: Index<Peer> = Index::unique(Namespace(4011), |r| {
        network_scoped_key(r.network_id, &r.public_key)
    });
    /// Peers by the peer that invited them; the owner's inviter is 0.
    pub const BY_INVITER: Index<Peer> = Index::new(Namespace(4012), |r| ref_key(r.inviter_peer_id));
}

impl Table for NetworkPeers {
    type Record = Peer;
    const NS: Namespace = Namespace(4009);
    const INDEXES: &'static [Index<Peer>] = &[
        NetworkPeers::BY_NETWORK,
        NetworkPeers::BY_PUBLIC_KEY,
        NetworkPeers::BY_INVITER,
    ];

    fn id(r: &Peer) -> u64 {
        r.id
    }
}

/// Invitations a new network peer may make.
pub const DEFAULT_INVITE_QUOTA: u32 = 3;

/// A member of network `network_id`, invited by peer `inviter_peer_id` (0
/// for the owner), joining at `now`.
pub fn new_network_peer(
    id: u64,
    network_id: u64,
    public_key: &[u8],
    alias: &str,
    inviter_peer_id: u64,
    now: Time,
) -> Peer {
    Peer {
        id,
        network_id,
        public_key: public_key.to_vec(),
        alias: alias.to_owned(),
        inviter_peer_id,
        invite_quota: DEFAULT_INVITE_QUOTA,
        created_at: now.unix(),
        ..Default::default()
    }
}

/// Aliases in networks whose CA this device runs: held by a peer's key,
/// or free once `reserved_until` (Unix seconds) has passed.
pub struct AliasReservations;

impl AliasReservations {
    /// Reservations by network.
    pub const BY_NETWORK: Index<AliasReservation> =
        Index::new(Namespace(4017), |r| ref_key(r.network_id));
    /// Reservations by (network, alias); at most one per alias.
    pub const BY_ALIAS: Index<AliasReservation> = Index::unique(Namespace(4018), |r| {
        network_scoped_key(r.network_id, r.alias.as_bytes())
    });
}

impl Table for AliasReservations {
    type Record = AliasReservation;
    const NS: Namespace = Namespace(4016);
    const INDEXES: &'static [Index<AliasReservation>] =
        &[AliasReservations::BY_NETWORK, AliasReservations::BY_ALIAS];

    fn id(r: &AliasReservation) -> u64 {
        r.id
    }
}

/// The `reserved_until` of an alias a peer holds.
pub const RESERVED_FOREVER: i64 = i64::MAX / 1_000_000_000;

/// `alias` on network `network_id`, held by `peer_key`.
pub fn new_alias_reservation(
    id: u64,
    network_id: u64,
    alias: &str,
    peer_key: &[u8],
) -> AliasReservation {
    AliasReservation {
        id,
        network_id,
        alias: alias.to_owned(),
        peer_key: peer_key.to_vec(),
        reserved_until: RESERVED_FOREVER,
    }
}

/// How the UI lists the profile's networks.
pub struct NetworkUiConfig;

impl Singleton for NetworkUiConfig {
    type Record = UiConfig;
    const NS: Namespace = Namespace(4008);

    fn default_value() -> Option<UiConfig> {
        Some(UiConfig::default())
    }

    fn observe(r: &UiConfig) -> Option<Record> {
        Some(Record::NetworkUiConfig(r.clone()))
    }
}

/// Whether this device accepts networks that peers publish to it.
pub struct BootstrapConfig;

impl Singleton for BootstrapConfig {
    type Record = bootstrap::Config;
    const NS: Namespace = Namespace(4015);

    fn default_value() -> Option<bootstrap::Config> {
        Some(bootstrap::Config::default())
    }

    fn observe(r: &bootstrap::Config) -> Option<Record> {
        Some(Record::BootstrapConfig(*r))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::expect_used, clippy::indexing_slicing)]

    use strims_proto::dao::v1::VersionVector;
    use strims_proto::network::v1::ServerConfig;
    use strims_proto::types::Certificate;

    use super::*;

    fn net(alias: &str, at: i64, not_after: u64, server: bool) -> Network {
        Network {
            id: 1,
            version: Some(VersionVector {
                value: [(1, 1)].into(),
                updated_at: at,
            }),
            certificate: Some(Certificate {
                key: b"root".to_vec(),
                not_after,
                ..Default::default()
            }),
            alias: alias.into(),
            server_config: server.then(ServerConfig::default),
            certificate_renewal_error: 0,
        }
    }

    #[test]
    fn concurrent_networks_resolve_the_same_way_on_both_sides() {
        let a = net("a", 10, 100, true);
        let b = net("b", 20, 50, false);
        let at_a = resolve_network(b.clone(), &a);
        let at_b = resolve_network(a.clone(), &b);
        assert_eq!(at_a.alias, "b");
        assert_eq!(at_a.certificate.as_ref().unwrap().not_after, 100);
        assert!(at_a.server_config.is_some());
        assert!(at_b.server_config.is_none());
        let strip = |mut n: Network| {
            n.server_config = None;
            n
        };
        assert_eq!(strip(at_a), strip(at_b));

        let tie_a = resolve_network(net("y", 5, 1, false), &net("x", 5, 1, false));
        let tie_b = resolve_network(net("x", 5, 1, false), &net("y", 5, 1, false));
        assert_eq!(tie_a, tie_b);
    }

    #[test]
    fn replicas_receive_networks_without_server_configs() {
        let rep = Networks::REPLICATION.unwrap();
        assert!((rep.extract)(net("a", 1, 1, true)).server_config.is_none());
        let merged = (rep.merge)(net("new", 2, 1, false), &net("old", 1, 1, true));
        assert_eq!(merged.alias, "new");
        assert!(merged.server_config.is_some());
    }

    #[test]
    fn a_network_holding_its_server_config_is_never_replaced() {
        let mut m = net("m", 1, 1, false);
        assert_eq!(
            Networks::on_unique_conflict(&Networks::BY_KEY, &mut m, &net("p", 2, 5, true)),
            UniqueConflict::Reject
        );
        assert_eq!(
            Networks::on_unique_conflict(&Networks::BY_KEY, &mut m, &net("p", 2, 5, false)),
            UniqueConflict::Replace
        );
        assert_eq!(m.alias, "p");
        assert_eq!(m.certificate.unwrap().not_after, 5);
    }
}
