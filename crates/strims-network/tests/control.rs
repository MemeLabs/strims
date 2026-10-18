// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Network controls on fake nodes (see `support`): binding, following the
//! store, joining, relinking, certificate renewal and its notifications,
//! and certificate lookups.

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
use strims_crypto::{certificate_parent, generate_key};
use strims_dao::events::Record;
use strims_dao::schema::network::{NetworkPeers, network_scoped_key};
use strims_dao::schema::notification::network_subject;
use strims_dao::{Recv, StoreEvent, Subscription, Table};
use strims_network::NetworkEvent;
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0, new_network};
use strims_network::services::RENEW_RETRY;
use strims_proto::network::v1::Network;
use strims_proto::network::v1::ca::ca_find_request::Query;
use strims_proto::network::v1::errors::ErrorCode;
use strims_proto::notification::v1::notification::Status;
use strims_proto::types::Key;
use support::*;

/// The next event that starts, joins or stops a network.
async fn next_lifecycle_event(events: &mut Subscription<NetworkEvent>) -> NetworkEvent {
    loop {
        let next = tokio::time::timeout(std::time::Duration::from_secs(60), events.next());
        match next.await.expect("an event within a minute") {
            Some(Recv::Event(e)) => match &*e {
                NetworkEvent::Updated { .. } | NetworkEvent::PeerCount { .. } => {}
                e => return e.clone(),
            },
            r => panic!("{r:?}"),
        }
    }
}

/// A network `owner` creates as network `id`, with each of `members`
/// invited to it as their network `id` and linked to the owner, once
/// their certificates are renewed: the network's key, and the links'
/// pipes.
async fn owned_network(owner: &Node, members: &[&Node], id: u64) -> (Key, Vec<Pipe>) {
    let mut rng = det_rng(id);
    let owned = new_network(id, "net", None, &owner.profile, "", T0, &mut rng).unwrap();
    let owner_cert = owned.certificate.clone().unwrap();
    let key = owned.server_config.as_ref().unwrap().key.clone().unwrap();
    owner.write_network(owned).await;
    let mut pipes = Vec::new();
    for m in members {
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(id, &invitation, &m.profile, "", T0, &mut rng).unwrap();
        m.write_network(joined).await;
        pipes.extend(link(owner, m));
    }
    settle().await;
    for m in members {
        let n = read_network(m, id).await;
        assert!(strims_crypto::membership_trusted(
            n.certificate.as_ref().unwrap(),
            T0
        ));
    }
    (key, pipes)
}

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

/// Nodes whose PSI runs on remote engines that answer late bind the
/// networks their profiles share, as inline PSI does.
#[test]
fn nodes_bind_through_late_remote_psi_engines() {
    run(async {
        let delay = std::time::Duration::from_millis(300);
        let (psi_a, handled_a) = remote_psi(delay);
        let (psi_b, handled_b) = remote_psi(delay);
        let a = node_with(&World::default(), 1, Some(psi_a)).await;
        let b = node_with(&World::default(), 2, Some(psi_b)).await;
        let mut rng = det_rng(10);
        let nets: Vec<Key> = (0..4).map(|_| generate_key(&mut rng)).collect();
        for (i, net) in nets[..3].iter().enumerate() {
            a.write_network(joined(10 + i as u64, net, &a)).await;
        }
        for (i, net) in nets[1..].iter().enumerate() {
            b.write_network(joined(20 + i as u64, net, &b)).await;
        }
        let _pipes = link(&a, &b);
        settle().await;
        assert_bound(&a, &b, &nets[1]);
        assert_bound(&a, &b, &nets[2]);
        assert_eq!(a.bound(&b, &nets[0]), None);
        assert_eq!(b.bound(&a, &nets[3]), None);
        for n in [&a, &b] {
            let m = n.control.metrics();
            assert_eq!((m.runs_completed, m.runs_failed), (1, 0), "{m:?}");
        }
        assert!(handled_a.get() > 0 && handled_b.get() > 0);
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
/// through the owner, whose CA signs it: then both bind, the CA records
/// the new peer under its inviter, and the invitee is notified.
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
        let notes = read_notifications(&invitee).await;
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(notes[0].status, Status::Success as i32);
        assert_eq!(notes[0].subject, Some(network_subject(80)));
        assert!(read_notifications(&owner).await.is_empty());

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

/// A network is reported joined once the node has joined it, with the
/// node's handle for it; deleted and held again, it starts over and is
/// joined again.
#[test]
fn a_network_reports_joined_after_the_node_joins_it() {
    run(async {
        let a = node(11).await;
        a.host.state.borrow_mut().join_delay = std::time::Duration::from_secs(1);
        let net = generate_key(&mut det_rng(110));
        let mut events = a.control.subscribe();
        a.write_network(joined(110, &net, &a)).await;
        let e = next_lifecycle_event(&mut events).await;
        assert!(
            matches!(&e, NetworkEvent::Started { network } if network.id == 110),
            "{e:?}"
        );
        assert_eq!(a.control.network(&net.public).unwrap().net, None);
        assert!(a.host.state.borrow().networks.is_empty());
        let e = next_lifecycle_event(&mut events).await;
        assert!(
            matches!(&e, NetworkEvent::Joined { network } if network.id == 110),
            "{e:?}"
        );
        let first = a.control.network(&net.public).unwrap().net;
        assert!(first.is_some());
        assert_eq!(
            first,
            a.host.state.borrow().networks.get(&net.public).copied()
        );

        a.delete_network(110).await;
        a.write_network(joined(111, &net, &a)).await;
        let got = [
            next_lifecycle_event(&mut events).await,
            next_lifecycle_event(&mut events).await,
            next_lifecycle_event(&mut events).await,
        ];
        assert!(
            matches!(
                &got,
                [
                    NetworkEvent::Stopped { network: stopped },
                    NetworkEvent::Started { network: started },
                    NetworkEvent::Joined { network: joined },
                ] if stopped.id == 110 && started.id == 111 && joined.id == 111
            ),
            "{got:?}"
        );
        let second = a.control.network(&net.public).unwrap().net;
        assert!(second.is_some() && second != first, "{first:?} {second:?}");
        assert_eq!(
            second,
            a.host.state.borrow().networks.get(&net.public).copied()
        );
        settle().await;
        while let Some(Some(Recv::Event(e))) = events.next().now_or_never() {
            assert!(
                matches!(*e, NetworkEvent::Updated { .. }),
                "joined once: {e:?}"
            );
        }
    });
}

/// A network the store deletes while the node joins it is reported
/// joined only if the store holds it again by the time the join ends:
/// then once, after its second start.
#[test]
fn a_network_deleted_while_joining_is_joined_only_if_held_again() {
    run(async {
        let a = node(18).await;
        a.host.state.borrow_mut().join_delay = std::time::Duration::from_secs(1);
        // the control follows the store from here
        settle().await;
        let mut rng = det_rng(180);
        let (kept, gone) = (generate_key(&mut rng), generate_key(&mut rng));
        let mut events = a.control.subscribe();
        a.write_network(joined(180, &kept, &a)).await;
        a.write_network(joined(181, &gone, &a)).await;
        a.delete_network(180).await;
        a.delete_network(181).await;
        a.write_network(joined(182, &kept, &a)).await;
        let mut got = Vec::new();
        for _ in 0..6 {
            got.push(match next_lifecycle_event(&mut events).await {
                NetworkEvent::Started { network } => ("started", network.id),
                NetworkEvent::Joined { network } => ("joined", network.id),
                NetworkEvent::Stopped { network } => ("stopped", network.id),
                e => panic!("{e:?}"),
            });
        }
        assert_eq!(
            got,
            [
                ("started", 180),
                ("started", 181),
                ("stopped", 180),
                ("stopped", 181),
                ("started", 182),
                ("joined", 182),
            ]
        );
        settle().await;
        while let Some(Some(Recv::Event(e))) = events.next().now_or_never() {
            assert!(matches!(*e, NetworkEvent::Updated { .. }), "{e:?}");
        }
        let net = a.control.network(&kept.public).unwrap().net;
        assert!(net.is_some());
        let host = a.host.state.borrow();
        assert_eq!(net, host.networks.get(&kept.public).copied());
        assert!(a.control.network(&gone.public).is_none());
        assert!(!host.networks.contains_key(&gone.public));
    });
}

/// A renewal that keeps failing the same way notifies once, and the
/// renewal that then succeeds replaces that notification.
#[test]
fn repeated_renewal_failures_notify_once() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 11).await;
        let member = node_in(&world, 12).await;
        let (_, _pipes) = owned_network(&owner, &[&member], 110).await;
        let renewed = read_notifications(&member).await;
        assert_eq!(renewed.len(), 1, "{renewed:?}");

        // the CA's address goes, so renewals over vpn can't reach it
        let addresses = std::mem::take(&mut world.0.borrow_mut().addresses);
        let mut events = member.store.events().subscribe();
        let n = read_network(&member, 110).await;
        member
            .write_network(Network {
                alias: "bob".into(),
                ..n
            })
            .await;
        settle().await;
        tokio::time::sleep((RENEW_RETRY * 3).to_std().unwrap()).await;
        let n = read_network(&member, 110).await;
        assert_eq!(n.certificate_renewal_error, ErrorCode::Unknown as i32);
        let failed = read_notifications(&member).await;
        assert_eq!(failed.len(), 1, "{failed:?}");
        assert_eq!(failed[0].status, Status::Error as i32);
        assert_eq!(failed[0].error_code, ErrorCode::Unknown as i32);
        assert_eq!(failed[0].subject, Some(network_subject(110)));
        assert!(!failed[0].message.is_empty());
        let mut raised = Vec::new();
        while let Some(Some(e)) = events.next().now_or_never() {
            match e {
                Recv::Event(e) => {
                    if let StoreEvent::Changed(Record::Notification(n)) = &*e {
                        raised.push(n.id);
                    }
                }
                Recv::Lagged => panic!("lagged"),
            }
        }
        assert_eq!(raised, [failed[0].id]);

        world.0.borrow_mut().addresses = addresses;
        tokio::time::sleep(RENEW_RETRY.to_std().unwrap()).await;
        settle().await;
        let n = read_network(&member, 110).await;
        assert_eq!(n.certificate.unwrap().subject, "bob");
        assert_eq!(n.certificate_renewal_error, 0);
        let notes = read_notifications(&member).await;
        assert_eq!(notes.len(), 1, "{notes:?}");
        assert_eq!(notes[0].status, Status::Success as i32);
        assert_ne!(notes[0].id, failed[0].id);
    });
}

/// A network the node fails to join starts, but isn't reported joined.
#[test]
fn a_network_the_node_fails_to_join_is_not_reported_joined() {
    run(async {
        let a = node(12).await;
        a.host.state.borrow_mut().refuse_networks = true;
        let net = generate_key(&mut det_rng(120));
        let mut events = a.control.subscribe();
        a.write_network(joined(120, &net, &a)).await;
        let e = next_lifecycle_event(&mut events).await;
        assert!(
            matches!(&e, NetworkEvent::Started { network } if network.id == 120),
            "{e:?}"
        );
        settle().await;
        while let Some(Some(Recv::Event(e))) = events.next().now_or_never() {
            assert!(!matches!(*e, NetworkEvent::Joined { .. }), "{e:?}");
        }
        assert_eq!(a.control.network(&net.public).unwrap().net, None);
        assert_eq!(a.control.metrics().host_errors, 1);
    });
}

/// The owner, holding the network's server config, finds certificates
/// its CA issued itself; a member finds the owner's and another
/// member's at the CA over vpn.
#[test]
fn find_certificate_answers_locally_and_over_vpn() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 13).await;
        let alice = node_in(&world, 14).await;
        let bob = node_in(&world, 15).await;
        let (key, _pipes) = owned_network(&owner, &[&alice, &bob], 130).await;
        let key = key.public;
        let owner_cert = read_network(&owner, 130).await.certificate.unwrap();
        let bob_cert = read_network(&bob, 130).await.certificate.unwrap();
        let by_subject = |n: &Node| Query::Subject(n.profile.name.clone());
        let owner_subject = Query::Subject(owner_cert.subject.clone());

        // over vpn: by subject and by key, and the owner's full chain
        let c = alice
            .control
            .find_certificate(&key, owner_subject.clone(), false)
            .await
            .unwrap();
        assert_eq!(c.key, owner.profile_key().public);
        let c = alice
            .control
            .find_certificate(&key, by_subject(&bob), false)
            .await
            .unwrap();
        assert_eq!(c.key, bob.profile_key().public);
        assert_eq!(c.serial_number, bob_cert.serial_number);
        let c = alice
            .control
            .find_certificate(&key, Query::Key(bob.profile_key().public.clone()), false)
            .await
            .unwrap();
        assert_eq!(c.subject, bob.profile.name);
        let chain = alice
            .control
            .find_certificate(&key, owner_subject.clone(), true)
            .await
            .unwrap();
        assert_eq!(chain, owner_cert);
        let e = alice
            .control
            .find_certificate(&key, Query::Subject("nobody".into()), false)
            .await
            .unwrap_err();
        assert_eq!(e.message, "not found");

        // with the CA's address withdrawn, the owner still answers from
        // its store, and a member can't reach the CA
        world.0.borrow_mut().addresses.clear();
        let c = owner
            .control
            .find_certificate(&key, by_subject(&alice), false)
            .await
            .unwrap();
        assert_eq!(c.key, alice.profile_key().public);
        let c = owner
            .control
            .find_certificate(&key, owner_subject, true)
            .await
            .unwrap();
        assert_eq!(c, owner_cert);
        let e = owner
            .control
            .find_certificate(&key, Query::Subject("nobody".into()), false)
            .await
            .unwrap_err();
        assert_eq!(e.message, "not found");
        let e = alice
            .control
            .find_certificate(&key, by_subject(&bob), false)
            .await
            .unwrap_err();
        assert_ne!(e.message, "not found");
        assert!(
            owner
                .control
                .find_certificate(b"no such network", by_subject(&bob), false)
                .await
                .is_err()
        );
    });
}

/// A host's certificate is its host key under the profile's current
/// membership, the one the CA finds by the profile's key, and follows a
/// renewal.
#[test]
fn a_host_certificate_is_under_the_current_membership() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 16).await;
        let member = node_in(&world, 17).await;
        let (key, _pipes) = owned_network(&owner, &[&member], 160).await;
        let key = key.public;
        let n = read_network(&member, 160).await;
        let membership = n.certificate.clone().unwrap();
        let host = member.control.host_certificate(&key).unwrap();
        assert_eq!(host.key, member.id.to_bytes());
        assert_eq!(certificate_parent(&host), Some(&membership));
        let found = owner
            .control
            .find_certificate(&key, Query::Key(member.profile_key().public.clone()), false)
            .await
            .unwrap();
        assert_eq!(found.serial_number, membership.serial_number);

        member
            .write_network(Network {
                alias: "carol".into(),
                ..n
            })
            .await;
        settle().await;
        let renewed = read_network(&member, 160).await.certificate.unwrap();
        assert_eq!(renewed.subject, "carol");
        let host = member.control.host_certificate(&key).unwrap();
        assert_eq!(certificate_parent(&host), Some(&renewed));
        assert_eq!(member.control.host_certificate(b"no such network"), None);
    });
}
