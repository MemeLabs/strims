// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Listing keepers on fake nodes (see `strims_testworld`): the first
//! publish, publishing again when the directory starts over, drops the
//! listing or lags without it, the pace of publishes, changed content and
//! the unpublish at the end.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration as StdDuration;

use futures_util::StreamExt;
use strims_base::rng::det_rng;
use strims_base::time::Duration;
use strims_dao::schema::network::Networks;
use strims_dao::{Recv, Table};
use strims_directory::{DirectoryEvent, KeeperConfig, KeeperMetrics, ListingEvent, ListingKeeper};
use strims_network::membership::{network_from_invitation_v0, new_invitation_v0};
use strims_proto::network::v1::Network;
use strims_proto::network::v1::directory::Listing;
use strims_proto::network::v1::directory::listing::{Chat, Content};
use strims_rpc::Timer;
use strims_testworld::*;
use tokio::task::JoinHandle;

fn chat(name: &str) -> Listing {
    Listing {
        content: Some(Content::Chat(Chat {
            key: vec![7; 32],
            name: name.into(),
        })),
    }
}

/// A keeper whose triggers are its events alone: checks are an hour
/// apart.
fn quiet() -> KeeperConfig {
    KeeperConfig {
        listing_check: Duration::from_secs(3600),
        ..KeeperConfig::default()
    }
}

/// A keeper running on a node.
struct Kept {
    keeper: ListingKeeper,
    /// The ids `on_listed` got.
    listed: Rc<RefCell<Vec<u64>>>,
    run: JoinHandle<()>,
}

/// Runs a keeper of `listing` on `node`'s directory of the network `key`,
/// fed the directory's events that `feed` lets through.
fn keep(
    node: &Node,
    key: &[u8],
    listing: Listing,
    config: KeeperConfig,
    feed: fn(&DirectoryEvent) -> bool,
) -> Kept {
    let keeper = ListingKeeper::new(key.to_vec(), listing);
    let listed = Rc::new(RefCell::new(Vec::new()));
    let mut events = node.directory.subscribe();
    let k = keeper.clone();
    node.spawn(async move {
        while let Some(e) = events.next().await {
            match e {
                Recv::Event(e) if feed(&e) => k.handle_event(&e),
                Recv::Event(_) => {}
                Recv::Lagged => k.check(),
            }
        }
    });
    let (k, d, l) = (keeper.clone(), node.directory.clone(), listed.clone());
    let run = node.spawn(async move { k.run(&d, config, |id| l.borrow_mut().push(id)).await });
    Kept {
        keeper,
        listed,
        run,
    }
}

fn every(_: &DirectoryEvent) -> bool {
    true
}

/// The id under which `n`'s view of network `network_id` lists `l`.
fn listed_as(n: &Node, network_id: u64, l: &Listing) -> Option<u64> {
    n.directory
        .listings(network_id)
        .into_iter()
        .find(|v| v.listing == *l)
        .map(|v| v.id)
}

fn publishes(k: &Kept) -> u64 {
    k.keeper.metrics().publishes
}

/// The keeper publishes its listing, every view lists it, and
/// `on_listed` gets its id.
#[test]
fn the_keeper_publishes_its_listing() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 40).await;
        let key = key.public;
        let kept = keep(&alice, &key, chat("room"), quiet(), every);
        wait_for(10, "the listing", || {
            listed_as(&owner, 40, &chat("room")).is_some()
                && listed_as(&alice, 40, &chat("room")).is_some()
        })
        .await;
        let id = listed_as(&owner, 40, &chat("room")).unwrap();
        assert_eq!(*kept.listed.borrow(), [id]);
        assert_eq!(kept.keeper.listing_id(), Some(id));
        assert_eq!(
            kept.keeper.metrics(),
            KeeperMetrics {
                publishes: 1,
                errors: 0
            }
        );
        sleep_secs(30).await;
        assert_eq!(publishes(&kept), 1);
    });
}

/// A keeper whose node doesn't run the network's directory waits,
/// without calls, and publishes once the node joins the network.
#[test]
fn the_keeper_waits_for_its_networks_directory() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[], 41).await;
        let key = key.public;
        let kept = keep(&alice, &key, chat("room"), quiet(), every);
        sleep_secs(30).await;
        assert_eq!(kept.keeper.metrics(), KeeperMetrics::default());
        assert_eq!(kept.keeper.listing_id(), None);

        let mut rng = det_rng(99);
        let owner_cert = owner.read_network(41).await.certificate.unwrap();
        let invitation = new_invitation_v0(&owner.profile, &owner_cert, T0, &mut rng).unwrap();
        let joined =
            network_from_invitation_v0(41, &invitation, &alice.profile, "", T0, &mut rng).unwrap();
        alice.write_network(joined).await;
        let _link = link(&owner, &alice);
        wait_for(120, "the listing", || {
            listed_as(&owner, 41, &chat("room")).is_some()
        })
        .await;
        let id = listed_as(&owner, 41, &chat("room")).unwrap();
        assert_eq!(*kept.listed.borrow(), [id]);
    });
}

/// When the node's directory of the network starts over (here as another
/// record of the network), the keeper publishes its listing again.
#[test]
fn the_keeper_publishes_again_when_its_directory_starts_over() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 42).await;
        let key = key.public;
        let kept = keep(&alice, &key, chat("room"), quiet(), every);
        wait_for(10, "the listing", || publishes(&kept) == 1).await;
        let id = kept.keeper.listing_id().unwrap();

        let record = alice.read_network(42).await;
        let mut tx = alice.store.write(alice.timer.now()).await.unwrap();
        Networks::delete(&mut tx, 42).await.unwrap();
        tx.commit().await.unwrap();
        wait_for(10, "the directory to stop", || {
            alice.directory.networks().is_empty()
        })
        .await;
        sleep_secs(10).await;
        assert_eq!(publishes(&kept), 1);
        alice.write_network(Network { id: 142, ..record }).await;
        wait_for(120, "the listing published again", || publishes(&kept) == 2).await;
        assert_eq!(kept.keeper.listing_id(), Some(id));
        assert_eq!(*kept.listed.borrow(), [id]);
        wait_for(330, "the listing in the view", || {
            listed_as(&alice, 142, &chat("room")) == Some(id)
        })
        .await;
    });
}

/// When the view drops the listing, the keeper publishes it again.
#[test]
fn the_keeper_publishes_again_when_the_view_drops_its_listing() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 43).await;
        let key = key.public;
        let kept = keep(&alice, &key, chat("room"), quiet(), every);
        wait_for(10, "the listing", || {
            listed_as(&owner, 43, &chat("room")).is_some()
        })
        .await;
        let id = kept.keeper.listing_id().unwrap();
        let mut events = alice.directory.subscribe();

        alice.directory.unpublish(&key, id).await.unwrap();
        wait_for(30, "the listing published again", || publishes(&kept) == 2).await;
        wait_for(10, "the listing in the views", || {
            [&owner, &alice]
                .iter()
                .all(|n| listed_as(n, 43, &chat("room")) == kept.keeper.listing_id())
        })
        .await;
        let got = drain(&mut events);
        assert!(
            got.iter().any(|e| matches!(e,
                DirectoryEvent::Listing { event: ListingEvent::Unpublished(l), .. } if l.id == id)),
            "{got:?}"
        );
        assert_eq!(
            kept.listed.borrow().last().copied(),
            kept.keeper.listing_id()
        );
    });
}

/// A lagged view that still lists the listing leaves it be; one that
/// lacks it has the keeper publish it again.
#[test]
fn the_keeper_publishes_again_when_a_lagged_view_lacks_its_listing() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 44).await;
        let key = key.public;
        // the keeper doesn't hear of the unpublish
        let kept = keep(&alice, &key, chat("room"), quiet(), |e| {
            !matches!(
                e,
                DirectoryEvent::Listing {
                    event: ListingEvent::Unpublished(_),
                    ..
                }
            )
        });
        wait_for(10, "the listing", || {
            listed_as(&alice, 44, &chat("room")).is_some()
        })
        .await;
        let id = kept.keeper.listing_id().unwrap();
        let lagged = DirectoryEvent::Lagged { network_id: 44 };

        kept.keeper.handle_event(&lagged);
        sleep_secs(10).await;
        assert_eq!(publishes(&kept), 1);

        alice.directory.unpublish(&key, id).await.unwrap();
        wait_for(10, "the view to drop the listing", || {
            listed_as(&alice, 44, &chat("room")).is_none()
        })
        .await;
        sleep_secs(10).await;
        assert_eq!(publishes(&kept), 1);

        kept.keeper
            .handle_event(&DirectoryEvent::Lagged { network_id: 45 });
        sleep_secs(10).await;
        assert_eq!(publishes(&kept), 1);

        kept.keeper.handle_event(&lagged);
        wait_for(1, "the listing published again", || publishes(&kept) == 2).await;
        wait_for(10, "the listing in the view", || {
            listed_as(&alice, 44, &chat("room")).is_some()
        })
        .await;
    });
}

/// Publishes are at least `min_republish` apart, and the reasons for one
/// that come meanwhile make one publish.
#[test]
fn the_keeper_publishes_at_most_once_per_min_republish() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 46).await;
        let key = key.public;
        let kept = keep(&alice, &key, chat("room"), quiet(), every);
        wait_for(10, "the listing", || publishes(&kept) == 1).await;
        let first = alice.timer.now();
        let started = DirectoryEvent::Started {
            network_id: 46,
            network_key: key.clone(),
        };
        let since = |d: &Node| (d.timer.now() - first).as_millis();

        sleep_secs(1).await;
        kept.keeper.handle_event(&started);
        sleep_secs(1).await;
        kept.keeper.handle_event(&started);
        let other = DirectoryEvent::Started {
            network_id: 47,
            network_key: vec![1; 32],
        };
        kept.keeper.handle_event(&other);
        tokio::time::sleep(StdDuration::from_millis(2800)).await;
        assert_eq!(publishes(&kept), 1, "{} ms after the first", since(&alice));
        wait_for(1, "the second publish", || publishes(&kept) == 2).await;

        sleep_secs(7).await;
        assert_eq!(publishes(&kept), 2);
        kept.keeper.handle_event(&started);
        wait_for(1, "the third publish", || publishes(&kept) == 3).await;
        assert_eq!(kept.keeper.metrics().errors, 0);
    });
}

/// A keeper given changed content unpublishes the listing, and publishes
/// the new content once the view has dropped the old: the directory keeps
/// a listed listing's content.
#[test]
fn a_changed_listing_is_unpublished_then_published_again() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 48).await;
        let key = key.public;
        let kept = keep(&alice, &key, chat("room"), quiet(), every);
        wait_for(10, "the listing", || {
            listed_as(&owner, 48, &chat("room")).is_some()
        })
        .await;
        let id = kept.keeper.listing_id().unwrap();
        let mut events = owner.directory.subscribe();

        kept.keeper.set_listing(chat("lobby"));
        wait_for(30, "the changed listing", || {
            [&owner, &alice].iter().all(|n| {
                listed_as(n, 48, &chat("lobby")).is_some()
                    && listed_as(n, 48, &chat("room")).is_none()
            })
        })
        .await;
        let got = drain(&mut events);
        let unpublished = got.iter().position(|e| {
            matches!(e, DirectoryEvent::Listing { event: ListingEvent::Unpublished(l), .. }
                if l.id == id && l.listing == chat("room"))
        });
        let changed = got.iter().position(|e| {
            matches!(e, DirectoryEvent::Listing { event: ListingEvent::Changed(l), .. }
                if l.listing == chat("lobby"))
        });
        assert!(unpublished.is_some() && unpublished < changed, "{got:?}");
        let now = listed_as(&owner, 48, &chat("lobby"));
        assert_eq!(kept.keeper.listing_id(), now);
        assert_eq!(kept.listed.borrow().last().copied(), now);
        assert_eq!(
            kept.keeper.metrics(),
            KeeperMetrics {
                publishes: 2,
                errors: 0
            }
        );

        kept.keeper.set_listing(chat("lobby"));
        sleep_secs(10).await;
        assert_eq!(publishes(&kept), 2);
    });
}

/// A stopped keeper unpublishes its listing and its run returns; one
/// stopped before its network's directory runs returns at once.
#[test]
fn a_stopped_keeper_unpublishes_its_listing() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (key, _pipes) = owned_network(&owner, &[&alice], 49).await;
        let key = key.public;
        let kept = keep(&alice, &key, chat("room"), quiet(), every);
        wait_for(10, "the listing", || {
            listed_as(&owner, 49, &chat("room")).is_some()
        })
        .await;

        kept.keeper.stop();
        tokio::time::timeout(StdDuration::from_secs(1), kept.run)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(kept.keeper.listing_id(), None);
        wait_for(10, "the listing to go", || {
            [&owner, &alice]
                .iter()
                .all(|n| listed_as(n, 49, &chat("room")).is_none())
        })
        .await;

        let idle = keep(&alice, &[9; 32], chat("room"), quiet(), every);
        sleep_secs(5).await;
        idle.keeper.stop();
        tokio::time::timeout(StdDuration::from_secs(1), idle.run)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(idle.keeper.metrics(), KeeperMetrics::default());
    });
}
