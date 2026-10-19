// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The mock streams a node reads; the crate docs' Reading section states
//! the contracts.

use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

use strims_base::time::{Duration, Time};
use strims_directory::Host;
use strims_kv::Kv;
use strims_ppspp::{SwarmId, SwarmOptions, Uri};
use strims_proto::debug::v1::MockStreamSegment;
use strims_proto::network::v1::directory::Listing;
use strims_proto::network::v1::directory::listing::Content;
use strims_rpc::Timer;
use strims_transfer::{ReadFailure, RecordReader, SwarmRead, TransferId};

use crate::control::{Shared, Stop};
use crate::producer::mock_stream_options;
use crate::{MAX_SEGMENT_BYTES, MOCK_FRAME, MOCK_STREAM_SERVICE, MOCK_WINDOW_BYTES};

/// The options a reader adds a mock stream's swarm with: the mock stream
/// options, with a [`MOCK_WINDOW_BYTES`] window.
pub(crate) fn reader_options() -> SwarmOptions {
    let o = mock_stream_options();
    SwarmOptions {
        live_window: MOCK_WINDOW_BYTES / o.chunk_size.max(1),
        ..o
    }
}

/// The swarm of `l`, if it is a mock stream's listing: a `Service`
/// listing of type [`MOCK_STREAM_SERVICE`] whose swarm URI parses and
/// carries the mock stream options.
pub(crate) fn mock_swarm(l: &Listing) -> Option<SwarmId> {
    let Some(Content::Service(s)) = &l.content else {
        return None;
    };
    if s.r#type != MOCK_STREAM_SERVICE {
        return None;
    }
    let uri = Uri::parse(&s.swarm_uri).ok()?;
    (uri.options == mock_stream_options().uri_options()).then_some(uri.id)
}

/// The mock streams to read of `listings`, the swarms of the mock stream
/// listings in the view of the network with key `network_key`, by listing
/// id: none unless `enabled` and the key is set, the lowest listing ids
/// first, a swarm once, and at most `max`. `skip` refuses a listing by its
/// id and swarm.
pub(crate) fn plan(
    enabled: bool,
    network_key: &[u8],
    listings: &BTreeMap<u64, SwarmId>,
    max: usize,
    skip: impl Fn(u64, &SwarmId) -> bool,
) -> BTreeMap<u64, SwarmId> {
    let mut wants = BTreeMap::new();
    if !enabled || network_key.is_empty() {
        return wants;
    }
    // Bound: `max` swarms.
    let mut swarms = BTreeSet::new();
    for (id, swarm) in listings {
        if wants.len() >= max {
            break;
        }
        if swarms.contains(swarm) || skip(*id, swarm) {
            continue;
        }
        swarms.insert(swarm.clone());
        wants.insert(*id, swarm.clone());
    }
    wants
}

/// A mock stream the node reads, by its network key and listing id.
pub(crate) struct Reader {
    pub(crate) swarm_id: SwarmId,
    /// Its transfer, once added.
    pub(crate) transfer: Option<TransferId>,
    pub(crate) started_at: Time,
    /// The stream's id, from its last segment.
    pub(crate) id: Option<u64>,
    pub(crate) segments: u64,
    pub(crate) malformed: u64,
    pub(crate) discontinuities: u64,
    /// The last segment's latency, and when it was read.
    pub(crate) latency: Option<Duration>,
    pub(crate) last_segment_at: Option<Time>,
    pub(crate) stop: Rc<Stop>,
}

/// How a reader ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum End {
    /// It was stopped.
    Stopped,
    /// Another component added the swarm's transfer first.
    Taken,
    /// Its swarm closed, or a read or node call failed.
    Failed,
}

/// A reader's key: its network key and listing id.
pub(crate) type ReaderKey = (Vec<u8>, u64);

impl<K: Kv, H: Host, T: Timer> Shared<K, H, T> {
    /// Runs the reader of `key` from its swarm's add to its end; see the
    /// crate docs.
    pub(crate) async fn run_reader(
        self: Rc<Self>,
        key: ReaderKey,
        swarm_id: SwarmId,
        stop: Rc<Stop>,
    ) {
        let end = self.read_swarm(&key, &swarm_id, &stop).await;
        tracing::debug!(listing = key.1, ?end, "mock stream reader ended");
        self.readers.borrow_mut().remove(&key);
        self.metrics.borrow_mut().readers_ended += 1;
        if end == End::Failed && !stop.is_stopped() {
            self.reader_ended(&key);
        }
        self.schedule_reconcile();
    }

    async fn read_swarm(&self, key: &ReaderKey, swarm_id: &SwarmId, stop: &Stop) -> End {
        // An add runs to its end, so the swarm it makes is always removed.
        let swarm = match self
            .host
            .add_swarm(swarm_id.clone(), reader_options())
            .await
        {
            Ok(s) => s,
            Err(e) => {
                self.host_failed("adding a mock stream swarm", &e);
                return End::Failed;
            }
        };
        if stop.is_stopped() {
            self.remove_host_swarm(swarm).await;
            return End::Stopped;
        }
        if self.transfers.find(swarm_id, b"").is_some() {
            self.remove_host_swarm(swarm).await;
            return End::Taken;
        }
        let t = self.transfers.add(swarm, swarm_id, Vec::new());
        if let Some(r) = self.readers.borrow_mut().get_mut(key) {
            r.transfer = Some(t);
        }
        self.publish_on_joined(t, &key.0);
        let end = match self.host.open_reader(swarm).await {
            Ok(r) => self.read_segments(key, r, stop).await,
            Err(e) => {
                self.host_failed("opening a mock stream reader", &e);
                End::Failed
            }
        };
        self.transfers.remove(t);
        self.remove_host_swarm(swarm).await;
        end
    }

    /// Reads segments until the reader stops, its swarm closes or a read
    /// fails.
    async fn read_segments<R: SwarmRead>(&self, key: &ReaderKey, r: R, stop: &Stop) -> End {
        let mut reader = match RecordReader::new(r, MOCK_FRAME, MAX_SEGMENT_BYTES) {
            Ok(r) => r,
            Err(e) => {
                self.metrics.borrow_mut().read_errors += 1;
                tracing::warn!(error = %e, "a mock stream's frame size was refused");
                return End::Failed;
            }
        };
        loop {
            let Some(res) = stop.until(reader.next_message::<MockStreamSegment>()).await else {
                return End::Stopped;
            };
            let m = reader.metrics();
            let segment = match res {
                Ok(s) => Some(s),
                Err(ReadFailure::Discontinuity) => None,
                Err(ReadFailure::Closed) => return End::Failed,
                Err(ReadFailure::Failed(e)) => {
                    self.metrics.borrow_mut().read_errors += 1;
                    tracing::debug!(error = %e, "reading a mock stream failed");
                    return End::Failed;
                }
            };
            let now = self.timer.now();
            if segment.is_some() {
                self.metrics.borrow_mut().segments_read += 1;
            }
            let mut readers = self.readers.borrow_mut();
            let Some(entry) = readers.get_mut(key) else {
                continue;
            };
            entry.malformed = m.malformed;
            entry.discontinuities = m.discontinuities;
            if let Some(s) = segment {
                let written = Time::from_unix_nanos(s.timestamp.saturating_mul(1000));
                entry.id = Some(s.id);
                entry.segments += 1;
                entry.latency = Some(now - written);
                entry.last_segment_at = Some(now);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use strims_proto::network::v1::directory::listing::{Media, Service};

    use super::*;

    fn uri(swarm: u8, options: &SwarmOptions) -> String {
        Uri::new(SwarmId::new(&[swarm; 32]), options.uri_options()).to_string()
    }

    fn service(kind: &str, swarm_uri: String) -> Listing {
        Listing {
            content: Some(Content::Service(Service {
                r#type: kind.into(),
                swarm_uri,
            })),
        }
    }

    fn swarm(n: u8) -> SwarmId {
        SwarmId::new(&[n; 32])
    }

    /// Listings `ids`, of the swarm each names.
    fn listings(ids: &[(u64, u8)]) -> BTreeMap<u64, SwarmId> {
        ids.iter().map(|(id, n)| (*id, swarm(*n))).collect()
    }

    fn ids(wants: &BTreeMap<u64, SwarmId>) -> Vec<u64> {
        wants.keys().copied().collect()
    }

    #[test]
    fn mock_stream_listings_carry_the_mock_options() {
        let mock = mock_stream_options();
        assert_eq!(
            mock_swarm(&service(MOCK_STREAM_SERVICE, uri(1, &mock))),
            Some(swarm(1))
        );
        let one_stream = SwarmOptions {
            stream_count: 1,
            ..mock_stream_options()
        };
        let media = Listing {
            content: Some(Content::Media(Media {
                mime_type: "video/mp4".into(),
                swarm_uri: uri(6, &mock),
            })),
        };
        for l in [
            service("other", uri(2, &mock)),
            service(MOCK_STREAM_SERVICE, uri(3, &one_stream)),
            service(MOCK_STREAM_SERVICE, "magnet:?xt=foo".into()),
            // A URI without its options.
            service(MOCK_STREAM_SERVICE, uri(5, &SwarmOptions::default())),
            media,
        ] {
            assert_eq!(mock_swarm(&l), None, "{l:?}");
        }
        // The options in another order.
        let reordered = uri(1, &mock).replace("&x.im=2", "") + "&x.im=2";
        assert_eq!(
            mock_swarm(&service(MOCK_STREAM_SERVICE, reordered)),
            Some(swarm(1))
        );
    }

    #[test]
    fn nothing_is_read_while_disabled_or_without_a_network() {
        let l = listings(&[(10, 1)]);
        assert!(plan(false, b"net", &l, 8, |_, _| false).is_empty());
        assert!(plan(true, b"", &l, 8, |_, _| false).is_empty());
        assert_eq!(ids(&plan(true, b"net", &l, 8, |_, _| false)), [10]);
    }

    #[test]
    fn the_lowest_listing_ids_are_read_up_to_the_bound() {
        let l = listings(&[(30, 3), (10, 1), (40, 4), (20, 2)]);
        let wants = plan(true, b"net", &l, 2, |_, _| false);
        assert_eq!(ids(&wants), [10, 20]);
        assert_eq!(wants[&10], swarm(1));
        assert!(plan(true, b"net", &l, 0, |_, _| false).is_empty());
    }

    #[test]
    fn skipped_listings_and_repeated_swarms_leave_room_for_others() {
        // 11 lists the swarm 10 lists.
        let l = listings(&[(10, 1), (11, 1), (12, 2), (13, 3)]);
        let own = swarm(2);
        let wants = plan(true, b"net", &l, 2, |_, s| *s == own);
        assert_eq!(ids(&wants), [10, 13]);
        let wants = plan(true, b"net", &l, 8, |id, _| id == 10);
        assert_eq!(ids(&wants), [11, 12, 13]);
    }

    #[test]
    fn readers_hold_the_mock_window() {
        let o = reader_options();
        assert_eq!(o.live_window * o.chunk_size, MOCK_WINDOW_BYTES);
        assert_eq!(o.uri_options(), mock_stream_options().uri_options());
    }
}
