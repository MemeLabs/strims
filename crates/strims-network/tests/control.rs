// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Network controls on fake nodes (see `support`): binding, following the
//! store, relinking, and certificate renewal.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::collections::BTreeMap;

use futures_util::{FutureExt, StreamExt};
use strims_base::rng::det_rng;
use strims_crypto::generate_key;
use strims_dao::Table;
use strims_dao::schema::network::{NetworkPeers, network_scoped_key};
use strims_network::NetworkEvent;
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0, new_network};
use strims_proto::network::v1::Network;
use strims_proto::types::Key;
use support::*;

/// Two nodes bind the networks their profiles share, and only those, and
/// report the bound peers.
#[test]
fn nodes_bind_the_networks_their_profiles_share() {
    run(async {
        let a = node(1).await;
        let b = node(2).await;
        let mut rng = det_rng(10);
        let nets: Vec<Key> = (0..4).map(|_| generate_key(&mut rng)).collect();
        for (i, net) in nets[..3].iter().enumerate() {
            a.write_network(joined(10 + i as u64, net, &a)).await;
        }
        for (i, net) in nets[1..].iter().enumerate() {
            b.write_network(joined(20 + i as u64, net, &b)).await;
        }
        let mut events = a.control.subscribe();
        let _pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &nets[1]);
        assert_bound(&a, &b, &nets[2]);
        assert_eq!(a.bound(&b, &nets[0]), None);
        assert_eq!(b.bound(&a, &nets[3]), None);

        let mut counts = BTreeMap::new();
        while let Some(Some(strims_dao::Recv::Event(e))) = events.next().now_or_never() {
            if let NetworkEvent::PeerCount { network_id, count } = *e {
                counts.insert(network_id, count);
            }
        }
        assert_eq!(counts, BTreeMap::from([(11, 1), (12, 1)]));
        let infos = a.control.networks();
        assert_eq!(infos.len(), 3);
        assert_eq!(
            infos.iter().map(|i| i.peer_count).sum::<usize>(),
            2,
            "{infos:?}"
        );
        let m = a.control.metrics();
        assert_eq!((m.runs_completed, m.runs_failed), (1, 0), "{m:?}");
    });
}

/// A network joined after the peers linked binds too; one deleted from
/// the store is closed and unbound on both sides.
#[test]
fn networks_follow_the_store() {
    run(async {
        let a = node(3).await;
        let b = node(4).await;
        let mut rng = det_rng(11);
        let nets: Vec<Key> = (0..2).map(|_| generate_key(&mut rng)).collect();
        a.write_network(joined(30, &nets[0], &a)).await;
        b.write_network(joined(40, &nets[0], &b)).await;
        let _pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &nets[0]);

        a.write_network(joined(31, &nets[1], &a)).await;
        b.write_network(joined(41, &nets[1], &b)).await;
        settle().await;
        assert_bound(&a, &b, &nets[1]);

        a.delete_network(30).await;
        settle().await;
        assert_eq!(a.bound(&b, &nets[0]), None);
        assert_eq!(b.bound(&a, &nets[0]), None);
        assert_bound(&a, &b, &nets[1]);
        assert!(!a.host.state.borrow().networks.contains_key(&nets[0].public));
    });
}

/// A peer that relinks negotiates again on the new link.
#[test]
fn a_relinked_peer_binds_again() {
    run(async {
        let a = node(5).await;
        let b = node(6).await;
        let net = generate_key(&mut det_rng(12));
        a.write_network(joined(50, &net, &a)).await;
        b.write_network(joined(60, &net, &b)).await;
        let pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &net);

        for p in pipes {
            p.close();
        }
        a.control.remove_peer(b.id);
        b.control.remove_peer(a.id);
        a.host.state.borrow_mut().bound.clear();
        b.host.state.borrow_mut().bound.clear();
        let _pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &net);
    });
}

/// A profile invited to its network renews its provisional certificate
/// through the owner, whose CA signs it: then both bind, and the CA
/// records the new peer under its inviter.
#[test]
fn an_invitee_renews_through_its_inviter_and_binds() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 7).await;
        let invitee = node_in(&world, 8).await;
        let mut rng = det_rng(70);
        let owned = new_network(70, "net", None, &owner.profile, "", T0, &mut rng).unwrap();
        let owner_cert = owned.certificate.clone().unwrap();
        let net_key = owned.server_config.as_ref().unwrap().key.clone().unwrap();
        owner.write_network(owned).await;
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(80, &invitation, &invitee.profile, "", T0, &mut rng)
                .unwrap();
        invitee.write_network(joined).await;

        let _pipes = link(&owner, &invitee);
        settle().await;
        assert_bound(&owner, &invitee, &net_key);
        let renewed = read_network(&invitee, 80).await;
        let cert = renewed.certificate.unwrap();
        assert!(strims_crypto::membership_trusted(&cert, T0));
        assert_eq!(renewed.certificate_renewal_error, 0);

        let mut tx = owner.store.read().await.unwrap();
        let scoped = |k: &[u8]| network_scoped_key(70, k);
        let owner_peer = NetworkPeers::index_get(
            &mut tx,
            &NetworkPeers::BY_PUBLIC_KEY,
            &scoped(&owner.profile_key().public),
        )
        .await
        .unwrap()
        .unwrap();
        let peer = NetworkPeers::index_get(
            &mut tx,
            &NetworkPeers::BY_PUBLIC_KEY,
            &scoped(&invitee.profile_key().public),
        )
        .await
        .unwrap()
        .unwrap();
        assert_eq!(peer.inviter_peer_id, owner_peer.id);
        assert_eq!(owner_peer.invite_quota, 2);
    });
}

/// A trusted member whose alias changes renews at the network's CA over
/// vpn, under the new alias.
#[test]
fn a_member_renews_at_the_ca_over_vpn() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 9).await;
        let member = node_in(&world, 10).await;
        let mut rng = det_rng(90);
        let owned = new_network(90, "net", None, &owner.profile, "", T0, &mut rng).unwrap();
        let owner_cert = owned.certificate.clone().unwrap();
        owner.write_network(owned).await;
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(100, &invitation, &member.profile, "", T0, &mut rng)
                .unwrap();
        member.write_network(joined).await;
        let _pipes = link(&owner, &member);
        settle().await;
        let n = read_network(&member, 100).await;
        assert!(strims_crypto::membership_trusted(
            n.certificate.as_ref().unwrap(),
            T0
        ));

        member
            .write_network(Network {
                alias: "bob".into(),
                ..n
            })
            .await;
        settle().await;
        let n = read_network(&member, 100).await;
        assert_eq!(n.certificate.unwrap().subject, "bob");
        assert_eq!(n.certificate_renewal_error, 0);
        let m = member.control.metrics();
        assert_eq!(m.runs_failed, 0, "{m:?}");
    });
}
