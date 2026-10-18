// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Change events.
//!
//! A store emits the events a write transaction collects once the
//! transaction commits, in the order the transaction produced them. Every
//! subscriber gets every event through its own queue of
//! [`SUBSCRIBER_CAPACITY`] events. A subscriber that falls that far
//! behind loses its queued events and receives [`Recv::Lagged`] once; it
//! then rereads what it needs from the store. Emitting never blocks and
//! never waits on a subscriber.
//!
//! [`Broadcast`] is the bus itself, for any event type; app services use
//! it for their own events.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use slotmap::{SlotMap, new_key_type};
use strims_proto::chat::v1 as chat;
use strims_proto::network::v1 as network;
use strims_proto::notification::v1::Notification;
use strims_proto::profile::v1::Device;
use strims_proto::replication::v1::{Checkpoint, EventLog};
use strims_proto::video::v1 as video;

/// The events a subscriber can hold before it lags.
pub const SUBSCRIBER_CAPACITY: usize = 256;

/// A record whose changes are observable.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum Record {
    /// A device of the profile.
    Device(Device),
    /// A replica's checkpoint.
    Checkpoint(Checkpoint),
    /// A network the profile belongs to.
    Network(network::Network),
    /// A bootstrap client.
    BootstrapClient(network::bootstrap::BootstrapClient),
    /// How the UI lists the profile's networks.
    NetworkUiConfig(network::UiConfig),
    /// Whether this device accepts published networks.
    BootstrapConfig(network::bootstrap::Config),
    /// The chat UI settings.
    ChatUiConfig(chat::UiConfig),
    /// A highlighted chat user.
    ChatUiConfigHighlight(chat::UiConfigHighlight),
    /// A tagged chat user.
    ChatUiConfigTag(chat::UiConfigTag),
    /// An ignored chat user.
    ChatUiConfigIgnore(chat::UiConfigIgnore),
    /// A whisper conversation.
    WhisperThread(chat::WhisperThread),
    /// A whisper message.
    WhisperRecord(chat::WhisperRecord),
    /// A chat server this device hosts.
    ChatServer(chat::Server),
    /// A chat server's icon.
    ChatServerIcon(chat::ServerIcon),
    /// A chat server's emote.
    ChatEmote(chat::Emote),
    /// A chat server's modifier.
    ChatModifier(chat::Modifier),
    /// A chat server's tag.
    ChatTag(chat::Tag),
    /// A notification for the user.
    Notification(Notification),
    /// A video channel.
    VideoChannel(video::VideoChannel),
    /// This device's video ingress config.
    VideoIngressConfig(video::VideoIngressConfig),
    /// This device's HLS egress config.
    HlsEgressConfig(video::HlsEgressConfig),
}

/// Something that happened in the store.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum StoreEvent {
    /// A record was written.
    Changed(Record),
    /// A record was deleted.
    Deleted(Record),
    /// A local transaction's replication log, with its records.
    EventLog(EventLog),
    /// Logs from the replica `from` advanced the store's checkpoint. Their
    /// records are stripped.
    LogsApplied {
        /// The replica the logs arrived from.
        from: u64,
        /// The logs.
        logs: Vec<EventLog>,
    },
}

/// What a [`Subscription`] yields.
#[derive(Clone, Debug, PartialEq)]
pub enum Recv<E = StoreEvent> {
    /// The next event.
    Event(Rc<E>),
    /// Events were dropped because the subscriber fell behind.
    Lagged,
}

new_key_type! {
    struct SubKey;
}

struct Sub<E> {
    queue: VecDeque<Rc<E>>,
    lagged: bool,
    waker: Option<Waker>,
}

impl<E> Default for Sub<E> {
    fn default() -> Self {
        Sub {
            queue: VecDeque::new(),
            lagged: false,
            waker: None,
        }
    }
}

/// Delivers events to subscribers, each through its own queue of
/// [`SUBSCRIBER_CAPACITY`] events. Clones share the subscriber list.
pub struct Broadcast<E>(Rc<RefCell<SlotMap<SubKey, Sub<E>>>>);

impl<E> Clone for Broadcast<E> {
    fn clone(&self) -> Self {
        Broadcast(self.0.clone())
    }
}

impl<E> Default for Broadcast<E> {
    fn default() -> Self {
        Broadcast(Rc::default())
    }
}

/// Delivers a store's events to its subscribers.
pub type EventBus = Broadcast<StoreEvent>;

impl<E> Broadcast<E> {
    /// A bus with no subscribers.
    pub fn new() -> Broadcast<E> {
        Broadcast::default()
    }

    /// Starts receiving the events emitted from now on. The subscribers
    /// are local services, so their number is the caller's to bound.
    pub fn subscribe(&self) -> Subscription<E> {
        let key = self.0.borrow_mut().insert(Sub::default());
        Subscription {
            bus: self.clone(),
            key,
        }
    }

    /// Queues `events` for every subscriber and wakes them. A subscriber
    /// whose queue is full loses it and lags.
    pub fn emit(&self, events: Vec<E>) {
        if events.is_empty() {
            return;
        }
        let events: Vec<Rc<E>> = events.into_iter().map(Rc::new).collect();
        let mut wake = Vec::new();
        {
            let mut subs = self.0.borrow_mut();
            for sub in subs.values_mut() {
                for e in &events {
                    if sub.queue.len() >= SUBSCRIBER_CAPACITY {
                        sub.queue.clear();
                        sub.lagged = true;
                    }
                    if !sub.lagged {
                        sub.queue.push_back(e.clone());
                    }
                }
                wake.extend(sub.waker.take());
            }
        }
        for w in wake {
            w.wake();
        }
    }
}

/// A subscriber's stream of events. Dropping it unsubscribes.
pub struct Subscription<E = StoreEvent> {
    bus: Broadcast<E>,
    key: SubKey,
}

impl<E> Stream for Subscription<E> {
    type Item = Recv<E>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Recv<E>>> {
        let mut subs = self.bus.0.borrow_mut();
        let Some(sub) = subs.get_mut(self.key) else {
            return Poll::Ready(None);
        };
        if sub.lagged {
            sub.lagged = false;
            return Poll::Ready(Some(Recv::Lagged));
        }
        match sub.queue.pop_front() {
            Some(e) => Poll::Ready(Some(Recv::Event(e))),
            None => {
                sub.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl<E> Drop for Subscription<E> {
    fn drop(&mut self) {
        let removed = self.bus.0.borrow_mut().remove(self.key);
        drop(removed);
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use futures_util::{FutureExt, StreamExt};

    use super::*;

    fn device(id: u64) -> StoreEvent {
        StoreEvent::Changed(Record::Device(Device {
            id,
            ..Default::default()
        }))
    }

    fn next(s: &mut Subscription) -> Option<Recv> {
        s.next().now_or_never().flatten()
    }

    #[test]
    fn every_subscriber_gets_every_event_in_order() {
        let bus = EventBus::new();
        let mut a = bus.subscribe();
        let mut b = bus.subscribe();
        bus.emit(vec![device(1), device(2)]);
        for s in [&mut a, &mut b] {
            assert_eq!(next(s), Some(Recv::Event(Rc::new(device(1)))));
            assert_eq!(next(s), Some(Recv::Event(Rc::new(device(2)))));
            assert_eq!(next(s), None);
        }
    }

    #[test]
    fn a_full_queue_lags_once_and_resumes() {
        let bus = EventBus::new();
        let mut slow = bus.subscribe();
        let mut fast = bus.subscribe();
        for i in 0..SUBSCRIBER_CAPACITY as u64 {
            bus.emit(vec![device(i)]);
            assert!(next(&mut fast).is_some());
        }
        bus.emit(vec![device(1000), device(1001)]);
        assert_eq!(next(&mut slow), Some(Recv::Lagged));
        assert_eq!(next(&mut slow), None);
        bus.emit(vec![device(2000)]);
        assert_eq!(next(&mut slow), Some(Recv::Event(Rc::new(device(2000)))));
        assert_eq!(next(&mut fast), Some(Recv::Event(Rc::new(device(1000)))));
    }

    #[test]
    fn dropped_subscriptions_stop_receiving() {
        let bus = EventBus::new();
        let s = bus.subscribe();
        drop(s);
        assert_eq!(bus.0.borrow().len(), 0);
        bus.emit(vec![device(1)]);
    }

    #[test]
    fn emit_wakes_a_waiting_subscriber() {
        let bus = EventBus::new();
        let mut s = bus.subscribe();
        let waker = futures_util::task::noop_waker();
        let mut cx = Context::from_waker(&waker);
        assert!(Pin::new(&mut s).poll_next(&mut cx).is_pending());
        assert!(bus.0.borrow().values().next().unwrap().waker.is_some());
        bus.emit(vec![device(1)]);
        assert!(bus.0.borrow().values().next().unwrap().waker.is_none());
    }
}
