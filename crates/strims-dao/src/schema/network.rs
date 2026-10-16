// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Networks the profile belongs to, and its bootstrap clients.

use prost::Message;
use strims_crypto::certificate_network_key;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::bootstrap::{BootstrapClient, bootstrap_client};

use crate::events::Record;
use crate::key::Namespace;
use crate::table::{
    ApplyConflict, Index, Replication, Table, UniqueConflict, resolve_last_written, upgrade_version,
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
