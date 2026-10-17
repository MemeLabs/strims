// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `OpenClient`: a client stream's bundles and events as the UI's
//! responses, with the listings the users in its messages view.

use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;
use prost::Message as _;
use strims_directory::{DirectoryControl, Host};
use strims_kv::Kv;
use strims_proto::chat::v1::message::DirectoryRef;
use strims_proto::chat::v1::open_client_response::{Body, Open, ServerEvents};
use strims_proto::chat::v1::{AssetBundle, OpenClientResponse, ServerEvent, server_event};
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::service::Error as RpcError;
use strims_rpc::Timer;

use super::ITEM_OVERHEAD;
use crate::assets::split;
use crate::{ClientEvent, ClientStream};

/// The listing the user with `peer_key` views: the first media or embed
/// listing a network's directory has them in.
fn viewed_listing<K: Kv, H: Host, T: Timer>(
    directory: &DirectoryControl<K, H, T>,
    peer_key: &[u8],
) -> Option<DirectoryRef> {
    if peer_key.is_empty() {
        return None;
    }
    directory
        .listings_by_peer_key(peer_key)
        .into_iter()
        .find_map(|(network_key, listings)| {
            let l = listings.into_iter().find(|l| {
                matches!(
                    l.listing.content,
                    Some(Content::Media(_) | Content::Embed(_))
                )
            })?;
            Some(DirectoryRef {
                directory_id: l.id,
                network_key,
                theme_color: l.snippet.as_ref().map_or(0, |s| s.theme_color),
                listing: Some(l.listing),
            })
        })
}

/// Sets the viewed listing of `e`'s message and of each of its nick
/// mentions, from `lookup`.
fn enrich(e: &mut ServerEvent, lookup: &mut impl FnMut(&[u8]) -> Option<DirectoryRef>) {
    let Some(server_event::Body::Message(m)) = &mut e.body else {
        return;
    };
    m.viewed_listing = lookup(&m.peer_key);
    for n in m.entities.iter_mut().flat_map(|es| es.nicks.iter_mut()) {
        n.viewed_listing = lookup(&n.peer_key);
    }
}

/// Clears the viewed listings [`enrich`] sets.
fn strip(e: &mut ServerEvent) {
    let Some(server_event::Body::Message(m)) = &mut e.body else {
        return;
    };
    m.viewed_listing = None;
    for n in m.entities.iter_mut().flat_map(|es| es.nicks.iter_mut()) {
        n.viewed_listing = None;
    }
}

/// An `OpenClient` call's stream: `Open`, then the client stream's bundles
/// and events, each response encoding to at most `max_bytes`. Dropping it
/// closes the client stream.
pub(super) struct OpenClient<K: Kv, H: Host, T: Timer> {
    stream: ClientStream<K, H, T>,
    directory: DirectoryControl<K, H, T>,
    max_bytes: usize,
    /// Bound: the responses of one item of the client stream.
    pending: VecDeque<OpenClientResponse>,
}

fn response(body: Body) -> OpenClientResponse {
    OpenClientResponse { body: Some(body) }
}

impl<K: Kv, H: Host, T: Timer> OpenClient<K, H, T> {
    pub(super) fn new(
        stream: ClientStream<K, H, T>,
        directory: DirectoryControl<K, H, T>,
        max_bytes: usize,
    ) -> Self {
        OpenClient {
            stream,
            directory,
            max_bytes,
            pending: VecDeque::from([response(Body::Open(Open {}))]),
        }
    }

    fn push_assets(&mut self, bundle: AssetBundle) {
        let s = split(bundle, self.max_bytes.saturating_sub(ITEM_OVERHEAD));
        if s.dropped > 0 {
            tracing::warn!(
                dropped = s.dropped,
                "chat asset bundle entries too large for a response"
            );
        }
        self.pending
            .extend(s.parts.into_iter().map(|b| response(Body::AssetBundle(b))));
    }

    fn push_events(&mut self, events: Vec<ServerEvent>) {
        // bound: the peer keys of one batch
        let mut viewed: BTreeMap<Vec<u8>, Option<DirectoryRef>> = BTreeMap::new();
        let directory = &self.directory;
        let mut lookup = |k: &[u8]| {
            viewed
                .entry(k.to_vec())
                .or_insert_with(|| viewed_listing(directory, k))
                .clone()
        };
        let budget = self.max_bytes.saturating_sub(ITEM_OVERHEAD);
        push_events(&mut self.pending, events, budget, &mut lookup);
    }
}

/// Queues `events`, with the viewed listings `lookup` gives, as
/// `ServerEvents` responses whose events encode to at most `budget` bytes,
/// counting [`ITEM_OVERHEAD`] for each. An event over the budget alone goes
/// without its viewed listings, or is left out if it's over it even so.
fn push_events(
    pending: &mut VecDeque<OpenClientResponse>,
    events: Vec<ServerEvent>,
    budget: usize,
    lookup: &mut impl FnMut(&[u8]) -> Option<DirectoryRef>,
) {
    let mut batch = Vec::new();
    let mut bytes = 0usize;
    for mut e in events {
        enrich(&mut e, lookup);
        let mut len = e.encoded_len().saturating_add(ITEM_OVERHEAD);
        if len > budget {
            strip(&mut e);
            len = e.encoded_len().saturating_add(ITEM_OVERHEAD);
        }
        if len > budget {
            tracing::warn!(len, "chat event too large for a response");
            continue;
        }
        if bytes.saturating_add(len) > budget {
            let events = std::mem::take(&mut batch);
            pending.push_back(response(Body::ServerEvents(ServerEvents { events })));
            bytes = 0;
        }
        bytes = bytes.saturating_add(len);
        batch.push(e);
    }
    if !batch.is_empty() {
        pending.push_back(response(Body::ServerEvents(ServerEvents { events: batch })));
    }
}

impl<K: Kv, H: Host, T: Timer> Stream for OpenClient<K, H, T> {
    type Item = Result<OpenClientResponse, RpcError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.get_mut();
        loop {
            if let Some(r) = this.pending.pop_front() {
                return Poll::Ready(Some(Ok(r)));
            }
            match Pin::new(&mut this.stream).poll_next(cx) {
                Poll::Ready(Some(ClientEvent::Assets(b))) => this.push_assets(b),
                Poll::Ready(Some(ClientEvent::Events(events))) => this.push_events(events),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

    use strims_proto::chat::v1::Message;
    use strims_proto::chat::v1::message::Entities;
    use strims_proto::chat::v1::message::entities::Nick;
    use strims_proto::network::v1::directory::Listing;
    use strims_proto::network::v1::directory::listing::Media;

    use super::*;

    fn event(peer: u8, body: &str, nicks: &[u8]) -> ServerEvent {
        ServerEvent {
            body: Some(server_event::Body::Message(Message {
                peer_key: vec![peer; 32],
                body: body.into(),
                entities: Some(Entities {
                    nicks: nicks
                        .iter()
                        .map(|n| Nick {
                            peer_key: vec![*n; 32],
                            ..Default::default()
                        })
                        .collect(),
                    ..Default::default()
                }),
                // what the server claims is replaced
                viewed_listing: Some(DirectoryRef::default()),
                ..Default::default()
            })),
        }
    }

    fn message(e: &ServerEvent) -> &Message {
        match &e.body {
            Some(server_event::Body::Message(m)) => m,
            None => panic!("no message"),
        }
    }

    /// Peer 1 views a media listing with a `len`-byte mime type; nobody
    /// else views one.
    fn lookup(len: usize) -> impl FnMut(&[u8]) -> Option<DirectoryRef> {
        move |k: &[u8]| {
            (k.first() == Some(&1)).then(|| DirectoryRef {
                directory_id: 5,
                network_key: vec![9; 32],
                listing: Some(Listing {
                    content: Some(Content::Media(Media {
                        mime_type: "x".repeat(len),
                        swarm_uri: String::new(),
                    })),
                }),
                theme_color: 0,
            })
        }
    }

    fn batches(pending: &VecDeque<OpenClientResponse>) -> Vec<Vec<ServerEvent>> {
        pending
            .iter()
            .map(|r| match &r.body {
                Some(Body::ServerEvents(e)) => e.events.clone(),
                b => panic!("not events: {b:?}"),
            })
            .collect()
    }

    #[test]
    fn messages_and_nicks_carry_what_their_users_view() {
        let mut pending = VecDeque::new();
        let events = vec![event(1, "a", &[2, 1]), event(2, "b", &[])];
        push_events(&mut pending, events, 4096, &mut lookup(8));
        let b = batches(&pending);
        assert_eq!(b.len(), 1);
        let (first, second) = (message(&b[0][0]), message(&b[0][1]));
        assert_eq!(first.viewed_listing.as_ref().unwrap().directory_id, 5);
        let nicks = &first.entities.as_ref().unwrap().nicks;
        assert_eq!(nicks[0].viewed_listing, None);
        assert_eq!(nicks[1].viewed_listing.as_ref().unwrap().directory_id, 5);
        assert_eq!(second.viewed_listing, None);
    }

    #[test]
    fn events_are_batched_within_the_budget() {
        let mut pending = VecDeque::new();
        let events: Vec<ServerEvent> = (0..10)
            .map(|i| event(2, &format!("{i:0>100}"), &[]))
            .collect();
        let len = events[0].encoded_len() + ITEM_OVERHEAD;
        push_events(&mut pending, events, 3 * len, &mut lookup(8));
        let sizes: Vec<usize> = batches(&pending).iter().map(Vec::len).collect();
        assert_eq!(sizes, [3, 3, 3, 1]);
        for r in &pending {
            assert!(r.encoded_len() <= 3 * len + ITEM_OVERHEAD);
        }
    }

    #[test]
    fn an_event_over_the_budget_goes_without_its_listings_or_not_at_all() {
        let mut pending = VecDeque::new();
        let plain = event(1, "big", &[1, 1]);
        let budget = plain.encoded_len() + ITEM_OVERHEAD + 100;
        let events = vec![plain, event(3, &"x".repeat(600), &[])];
        push_events(&mut pending, events, budget, &mut lookup(200));
        let b = batches(&pending);
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].len(), 1);
        let m = message(&b[0][0]);
        assert_eq!(m.body, "big");
        assert_eq!(m.viewed_listing, None);
        let nicks = &m.entities.as_ref().unwrap().nicks;
        assert!(nicks.iter().all(|n| n.viewed_listing.is_none()));
    }
}
