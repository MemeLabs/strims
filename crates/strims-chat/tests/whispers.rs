// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Whisper controls on fake nodes (see `strims_testworld`): delivery and
//! its retries, the receiver's refusals and bounds, duplicates, rates and
//! the whisper lease.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use strims_chat::rate::RateConfig;
use strims_chat::whisper::WhisperLimits;
use strims_chat::{EnqueueError, WHISPER_SALT, WhisperConfig, WhisperControl};
use strims_dao::Table;
use strims_dao::schema::chat::{ChatUiConfigIgnores, WhisperRecords, WhisperThreads};
use strims_dao::schema::network::Networks;
use strims_kv::MemoryKv;
use strims_proto::chat::v1::{MessageState, UiConfigIgnore, WhisperRecord, WhisperThread};
use strims_testworld::*;

type Whispers = WhisperControl<MemoryKv, FakeHost, TokioTimer>;

/// Runs a whisper control on `n`.
fn start_with(n: &Node, config: WhisperConfig) -> Whispers {
    let w = WhisperControl::new(
        n.host.clone(),
        n.store.clone(),
        n.profile.clone(),
        n.network.clone(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let c = w.clone();
    n.spawn(async move { c.run().await });
    w
}

fn start(n: &Node) -> Whispers {
    start_with(n, WhisperConfig::default())
}

/// A config whose senders may send `burst` whispers at once.
fn burst(burst: u32) -> WhisperConfig {
    WhisperConfig {
        rate: RateConfig {
            burst,
            ..RateConfig::default()
        },
        ..WhisperConfig::default()
    }
}

fn key(n: &Node) -> Vec<u8> {
    n.profile_key().public.clone()
}

/// `n`'s alias on network `id`.
async fn alias(n: &Node, id: u64) -> String {
    n.read_network(id).await.certificate.unwrap().subject
}

async fn send(from: &Whispers, net: &[u8], to: &Node, body: &str) -> u64 {
    from.enqueue(net, b"", &key(to), "them", body)
        .await
        .unwrap()
        .id
}

async fn records(n: &Node) -> Vec<WhisperRecord> {
    let mut tx = n.store.read().await.unwrap();
    WhisperRecords::get_all(&mut tx).await.unwrap()
}

async fn record(n: &Node, id: u64) -> WhisperRecord {
    let mut tx = n.store.read().await.unwrap();
    WhisperRecords::get(&mut tx, id).await.unwrap().unwrap()
}

async fn thread(n: &Node, peer_key: &[u8]) -> Option<WhisperThread> {
    let mut tx = n.store.read().await.unwrap();
    WhisperThreads::index_get(&mut tx, &WhisperThreads::BY_PEER_KEY, peer_key)
        .await
        .unwrap()
}

async fn state(n: &Node, id: u64) -> MessageState {
    MessageState::try_from(record(n, id).await.state).unwrap()
}

fn count(records: &[WhisperRecord], state: MessageState) -> usize {
    records.iter().filter(|r| r.state == state as i32).count()
}

/// Waits up to `secs` seconds for record `id` of `n` to be in `want`.
async fn wait_state(n: &Node, id: u64, want: MessageState, secs: u64) {
    for _ in 0..secs * 10 {
        if state(n, id).await == want {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!(
        "whisper {id} is {:?}, not {want:?}, after {secs} s",
        state(n, id).await
    );
}

/// Waits up to `secs` seconds until none of `n`'s records is ENQUEUED.
async fn wait_settled(n: &Node, secs: u64) -> Vec<WhisperRecord> {
    for _ in 0..secs * 10 {
        let rs = records(n).await;
        if count(&rs, MessageState::Enqueued) == 0 {
            return rs;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    panic!("whispers still enqueued after {secs} s");
}

/// Sets `n`'s record `id` to `state`.
async fn set_state(n: &Node, id: u64, state: MessageState) {
    let mut tx = n.store.write(T0).await.unwrap();
    let mut r = WhisperRecords::get(&mut tx, id).await.unwrap().unwrap();
    r.state = state as i32;
    WhisperRecords::update(&mut tx, r).await.unwrap();
    tx.commit().await.unwrap();
}

/// A whisper reaches its recipient: the sender's record is DELIVERED, the
/// recipient's is UNREAD with the sender's identity, and both threads
/// follow it. Marking it read clears the thread.
#[test]
fn a_whisper_is_delivered() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let wo = start(&owner);
        let wa = start(&alice);
        sleep_secs(1).await;
        assert!(wo.holds_lease() && wa.holds_lease());
        assert!(wo.published(&net) && wa.published(&net));
        assert_eq!(
            world.address(&key(&owner), WHISPER_SALT).map(|a| a.host),
            Some(owner.id)
        );

        let body = "hi, see strims.gg";
        let sent = wa
            .enqueue(&net, &[9; 32], &key(&owner), "the owner", body)
            .await
            .unwrap();
        assert_eq!(sent.state, MessageState::Enqueued as i32);
        let m = sent.message.as_ref().unwrap();
        assert_eq!(m.peer_key, key(&alice));
        assert_eq!(m.nick, alias(&alice, 10).await);
        assert_eq!(m.entities.as_ref().unwrap().links.len(), 1);
        let t = thread(&alice, &key(&owner)).await.unwrap();
        assert_eq!(
            (t.alias.as_str(), t.last_message_id),
            ("the owner", sent.id)
        );
        assert!(!t.has_unread);

        wait_state(&alice, sent.id, MessageState::Delivered, 5).await;
        let got = records(&owner).await;
        assert_eq!(got.len(), 1, "{got:?}");
        let r = &got[0];
        assert_eq!(r.state, MessageState::Unread as i32);
        assert_eq!(r.network_key, net);
        assert_eq!(r.server_key, vec![9; 32]);
        assert_eq!(r.peer_key, key(&alice));
        assert_eq!(r.peer_message_id, sent.id);
        let m = r.message.as_ref().unwrap();
        assert_eq!(m.body, body);
        assert_eq!(m.peer_key, key(&alice));
        assert_eq!(m.nick, alias(&alice, 10).await);
        assert_eq!(
            m.entities.as_ref().unwrap().links[0].url,
            "https://strims.gg"
        );
        let t = thread(&owner, &key(&alice)).await.unwrap();
        assert!(t.has_unread);
        assert_eq!(t.last_message_id, r.id);
        assert_eq!(t.alias, alias(&alice, 10).await);
        let (ma, mo) = (wa.metrics(), wo.metrics());
        assert_eq!((ma.attempts, ma.delivered, ma.pending), (1, 1, 0), "{ma:?}");
        assert_eq!((mo.received, mo.refused), (1, 0), "{mo:?}");

        wo.mark_read(&key(&alice)).await.unwrap();
        assert_eq!(state(&owner, r.id).await, MessageState::Read);
        assert!(!thread(&owner, &key(&alice)).await.unwrap().has_unread);
    });
}

/// A recipient that isn't running yet gets the whisper at a retry once it
/// is.
#[test]
fn a_whisper_waits_for_its_recipient() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let wa = start(&alice);
        sleep_secs(1).await;
        let id = send(&wa, &net, &owner, "are you there").await;
        sleep_secs(5).await;
        assert_eq!(state(&alice, id).await, MessageState::Enqueued);
        assert_eq!(wa.metrics().attempts, 1);

        let wo = start(&owner);
        wait_state(&alice, id, MessageState::Delivered, 15).await;
        assert_eq!(wa.metrics().attempts, 2);
        assert_eq!(wo.metrics().received, 1);
        assert_eq!(records(&owner).await.len(), 1);
    });
}

/// A whisper whose recipient never publishes an address fails after its
/// last attempt, and no attempt follows.
#[test]
fn an_unreachable_whisper_fails_after_its_attempts() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let wa = start(&alice);
        sleep_secs(1).await;
        let id = send(&wa, &net, &owner, "hello?").await;
        sleep_secs(500).await;
        assert_eq!(state(&alice, id).await, MessageState::Enqueued);
        assert_eq!(wa.metrics().attempts, 5);

        wait_state(&alice, id, MessageState::Failed, 60).await;
        sleep_secs(600).await;
        let m = wa.metrics();
        assert_eq!((m.attempts, m.failed, m.pending), (6, 1, 0), "{m:?}");
    });
}

/// An ignored sender's whisper is refused, and fails without a retry.
#[test]
fn an_ignored_senders_whisper_fails() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let mut tx = owner.store.write(T0).await.unwrap();
        let ignore = UiConfigIgnore {
            id: tx.generate_id().await.unwrap(),
            alias: alias(&alice, 10).await,
            peer_key: key(&alice),
            deadline: 0,
            ..Default::default()
        };
        ChatUiConfigIgnores::insert(&mut tx, ignore).await.unwrap();
        tx.commit().await.unwrap();
        let wo = start(&owner);
        let wa = start(&alice);
        sleep_secs(1).await;

        let id = send(&wa, &net, &owner, "let me in").await;
        wait_state(&alice, id, MessageState::Failed, 2).await;
        sleep_secs(60).await;
        assert_eq!(wa.metrics().attempts, 1);
        assert_eq!(wo.metrics().refused, 1);
        assert!(records(&owner).await.is_empty());
        assert!(thread(&owner, &key(&alice)).await.is_none());
    });
}

/// A recipient takes 15 unread whispers from a sender and refuses the
/// 16th; once it reads them, it takes more.
#[test]
fn a_sender_has_fifteen_unread_whispers_at_most() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let wo = start_with(&owner, burst(100));
        let wa = start(&alice);
        sleep_secs(1).await;

        for i in 0..16 {
            send(&wa, &net, &owner, &format!("whisper {i}")).await;
        }
        let sent = wait_settled(&alice, 5).await;
        let sent: Vec<WhisperRecord> = sent
            .into_iter()
            .filter(|r| r.peer_key == key(&owner))
            .collect();
        assert_eq!(count(&sent, MessageState::Delivered), 15);
        assert_eq!(count(&sent, MessageState::Failed), 1);
        let got = records(&owner).await;
        assert_eq!(count(&got, MessageState::Unread), 15);
        assert_eq!(got.len(), 15);
        assert_eq!(wo.metrics().refused, 1);

        wo.mark_read(&key(&alice)).await.unwrap();
        let id = send(&wa, &net, &owner, "one more").await;
        wait_state(&alice, id, MessageState::Delivered, 5).await;
        assert_eq!(count(&records(&owner).await, MessageState::Unread), 1);
    });
}

/// A whisper sent again under the same id is taken once: the second
/// delivery succeeds without a second record.
#[test]
fn a_whisper_sent_twice_is_stored_once() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let wo = start(&owner);
        let wa = start(&alice);
        sleep_secs(1).await;
        let id = send(&wa, &net, &owner, "once").await;
        wait_state(&alice, id, MessageState::Delivered, 5).await;

        set_state(&alice, id, MessageState::Enqueued).await;
        wait_state(&alice, id, MessageState::Delivered, 5).await;
        assert_eq!(wa.metrics().attempts, 2);
        assert_eq!(records(&owner).await.len(), 1);
        let m = wo.metrics();
        assert_eq!((m.received, m.duplicates), (1, 1), "{m:?}");
    });
}

/// At the thread cap a whisper from a sender without a thread is refused,
/// and one from a sender with a thread is taken.
#[test]
fn new_threads_stop_at_the_cap() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let bob = node_in(&world, 3).await;
        let (root, _pipes) = owned_network(&owner, &[&alice, &bob], 10).await;
        let net = root.public;
        let mut tx = owner.store.write(T0).await.unwrap();
        let other = WhisperThread {
            id: tx.generate_id().await.unwrap(),
            peer_key: vec![5; 32],
            alias: "someone".into(),
            ..Default::default()
        };
        WhisperThreads::insert(&mut tx, other).await.unwrap();
        tx.commit().await.unwrap();
        let config = WhisperConfig {
            limits: WhisperLimits {
                max_threads: 2,
                ..WhisperLimits::default()
            },
            ..WhisperConfig::default()
        };
        let wo = start_with(&owner, config);
        let wa = start(&alice);
        let wb = start(&bob);
        sleep_secs(1).await;

        let a1 = send(&wa, &net, &owner, "first").await;
        wait_state(&alice, a1, MessageState::Delivered, 5).await;
        let b1 = send(&wb, &net, &owner, "too late").await;
        wait_state(&bob, b1, MessageState::Failed, 5).await;
        let a2 = send(&wa, &net, &owner, "second").await;
        wait_state(&alice, a2, MessageState::Delivered, 5).await;
        assert!(thread(&owner, &key(&bob)).await.is_none());
        assert_eq!(records(&owner).await.len(), 2);
        assert_eq!(wo.metrics().refused, 1);
    });
}

/// A sender over its rate is answered "sending too fast", and its
/// whispers land at their retries.
#[test]
fn senders_over_their_rate_retry() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let wo = start(&owner);
        let wa = start(&alice);
        sleep_secs(1).await;

        for i in 0..7 {
            send(&wa, &net, &owner, &format!("quick {i}")).await;
        }
        sleep_secs(2).await;
        let sent = records(&alice).await;
        assert_eq!(count(&sent, MessageState::Delivered), 5);
        assert_eq!(count(&sent, MessageState::Enqueued), 2);
        assert_eq!(wo.metrics().rate_limited, 2);

        let sent = wait_settled(&alice, 15).await;
        assert_eq!(count(&sent, MessageState::Delivered), 7);
        assert_eq!(records(&owner).await.len(), 7);
        assert_eq!(wa.metrics().attempts, 9);
    });
}

/// Of two processes sharing a profile's store, the lease holder alone
/// publishes and delivers; when it ends, the other takes over.
#[test]
fn one_process_of_a_shared_store_runs_whispers() {
    run(async {
        let world = World::default();
        let first = node_in(&world, 1).await;
        let second = node_sharing(&world, 4, &first);
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&first, &[&alice], 14).await;
        let net = root.public;
        let w1 = start(&first);
        let w2 = start(&second);
        let wa = start(&alice);
        sleep_secs(1).await;
        let holding = [w1.holds_lease(), w2.holds_lease()];
        assert_eq!(holding.iter().filter(|h| **h).count(), 1, "{holding:?}");
        let (holder, hw, other, ow) = if holding[0] {
            (&first, &w1, &second, &w2)
        } else {
            (&second, &w2, &first, &w1)
        };
        assert!(hw.published(&net) && !ow.published(&net));
        assert_eq!(
            world.address(&key(&first), WHISPER_SALT).map(|a| a.host),
            Some(holder.id)
        );

        let id = send(ow, &net, &alice, "from the profile").await;
        wait_state(&first, id, MessageState::Delivered, 5).await;
        let id = send(&wa, &net, &first, "to the profile").await;
        wait_state(&alice, id, MessageState::Delivered, 5).await;
        let (h, o) = (hw.metrics(), ow.metrics());
        assert_eq!((h.attempts, h.received), (1, 1), "{h:?}");
        assert_eq!((o.attempts, o.received, o.enqueued), (0, 0, 1), "{o:?}");

        holder.stop();
        wait_for(60, "the other process to take the lease", || {
            ow.holds_lease()
        })
        .await;
        sleep_secs(1).await;
        assert!(ow.published(&net));
        assert_eq!(
            world.address(&key(&first), WHISPER_SALT).map(|a| a.host),
            Some(other.id)
        );
        let id = send(ow, &net, &alice, "still here").await;
        wait_state(&first, id, MessageState::Delivered, 5).await;
        assert_eq!(ow.metrics().attempts, 1);
    });
}

/// Leaving a network closes its whisper port and withdraws its address.
#[test]
fn the_address_goes_with_the_network() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let wa = start(&alice);
        sleep_secs(1).await;
        assert!(wa.published(&net));
        assert!(world.address(&key(&alice), WHISPER_SALT).is_some());

        let mut tx = alice.store.write(T0).await.unwrap();
        Networks::delete(&mut tx, 10).await.unwrap();
        tx.commit().await.unwrap();
        sleep_secs(1).await;
        assert!(!wa.published(&net));
        assert_eq!(world.address(&key(&alice), WHISPER_SALT), None);
    });
}

/// What can't be sent isn't stored.
#[test]
fn enqueue_refuses_what_it_cannot_send() {
    run(async {
        let world = World::default();
        let owner = node_in(&world, 1).await;
        let alice = node_in(&world, 2).await;
        let (root, _pipes) = owned_network(&owner, &[&alice], 10).await;
        let net = root.public;
        let config = WhisperConfig {
            limits: WhisperLimits {
                max_pending: 2,
                ..WhisperLimits::default()
            },
            ..WhisperConfig::default()
        };
        let wa = start_with(&alice, config);
        sleep_secs(1).await;
        let to = key(&owner);
        let err = |r: Result<WhisperRecord, EnqueueError>| r.unwrap_err();

        assert_eq!(
            err(wa.enqueue(&[1; 32], b"", &to, "x", "hi").await),
            EnqueueError::NoNetwork
        );
        assert_eq!(
            err(wa.enqueue(&net, b"", &key(&alice), "me", "hi").await),
            EnqueueError::ToSelf
        );
        let long = "x".repeat(WhisperLimits::default().max_body_bytes + 1);
        assert_eq!(
            err(wa.enqueue(&net, b"", &to, "x", &long).await),
            EnqueueError::TooLong
        );
        assert_eq!(
            err(wa.enqueue(&net, b"", &[], "x", "hi").await),
            EnqueueError::TooLarge
        );
        wa.enqueue(&net, b"", &to, "x", "one").await.unwrap();
        wa.enqueue(&net, b"", &to, "x", "two").await.unwrap();
        assert_eq!(
            err(wa.enqueue(&net, b"", &to, "x", "three").await),
            EnqueueError::Full
        );
        assert_eq!(records(&alice).await.len(), 2);
        assert_eq!(wa.metrics().enqueued, 2);
    });
}
