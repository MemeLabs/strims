// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The mock streams a node writes; the crate docs' Format and Producing
//! sections state the contracts.

use std::rc::Rc;

use futures_util::FutureExt;
use prost::Message;
use strims_base::time::{Duration, Time};
use strims_directory::{Host, ListingKeeper};
use strims_kv::Kv;
use strims_ppspp::integrity::{
    LiveSignatureAlgorithm, MerkleHashTreeFunction, ProtectionMethod, VerifierOptions,
};
use strims_ppspp::{SwarmOptions, Uri, WriterOptions};
use strims_proto::debug::v1::MockStreamSegment;
use strims_proto::network::v1::directory::Listing;
use strims_proto::network::v1::directory::listing::{Content, Service};
use strims_proto::types::Key;
use strims_rpc::Timer;
use strims_transfer::{RecordWriter, SwarmWrite, TransferId};

use crate::control::{Reply, Shared, Stop};
use crate::{Config, DebugError, MAX_SEGMENT_BYTES, MOCK_FRAME, MOCK_STREAM_SERVICE};

/// The options of a mock stream's swarm; its URI carries every one but the
/// window.
pub fn mock_stream_options() -> SwarmOptions {
    SwarmOptions {
        chunk_size: 1024,
        chunks_per_signature: 32,
        stream_count: 16,
        live_window: 16 * 1024,
        integrity: VerifierOptions {
            protection_method: ProtectionMethod::MERKLE_TREE,
            merkle_hash_tree_function: MerkleHashTreeFunction::BLAKE2B256,
            live_signature_algorithm: LiveSignatureAlgorithm::ED25519,
        },
        ..SwarmOptions::default()
    }
    .assign_defaults(&SwarmOptions::default_options())
}

/// The padding of a segment of a `bitrate_kbps` stream written every
/// `segment_interval_ms`: the bitrate's bytes for one interval.
pub fn padding_len(bitrate_kbps: u32, segment_interval_ms: u32) -> usize {
    let bytes = u64::from(bitrate_kbps) * 125 * u64::from(segment_interval_ms) / 1000;
    usize::try_from(bytes).unwrap_or(usize::MAX)
}

/// The most bytes a segment with `padding` bytes of padding encodes to:
/// with the longest id and timestamp varints.
pub(crate) fn segment_bound(padding: usize) -> usize {
    // A negative timestamp's varint is the longest, as is u64::MAX's.
    let fields = MockStreamSegment {
        id: u64::MAX,
        timestamp: -1,
        padding: Vec::new(),
    }
    .encoded_len();
    if padding == 0 {
        return fields;
    }
    // The padding's tag, its length's varint and its bytes.
    fields
        .saturating_add(1 + prost::encoding::encoded_len_varint(padding as u64))
        .saturating_add(padding)
}

/// A mock stream request, checked against the bounds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Start {
    pub(crate) bitrate_kbps: u32,
    /// The interval, the default for a request of 0.
    pub(crate) segment_interval_ms: u32,
    /// 0 runs the stream until it is stopped.
    pub(crate) timeout_ms: u32,
    pub(crate) padding: usize,
}

impl Start {
    pub(crate) fn interval(&self) -> Duration {
        Duration::from_millis(i64::from(self.segment_interval_ms))
    }

    /// When the stream started at `started_at` times out, if it does.
    pub(crate) fn deadline(&self, started_at: Time) -> Option<Time> {
        (self.timeout_ms > 0)
            .then(|| started_at + Duration::from_millis(i64::from(self.timeout_ms)))
    }
}

/// Checks a mock stream request against `c`: a bitrate of at most
/// [`Config::max_bitrate_kbps`], a segment interval (0 for the default)
/// within its bounds, a timeout (0 for none) of at most
/// [`Config::max_timeout`], and segments of at most [`MAX_SEGMENT_BYTES`].
pub(crate) fn check_start(
    c: &Config,
    bitrate_kbps: u32,
    segment_interval_ms: u32,
    timeout_ms: u32,
) -> Result<Start, DebugError> {
    if bitrate_kbps > c.max_bitrate_kbps {
        return Err(DebugError::BitrateOutOfRange);
    }
    let interval = match segment_interval_ms {
        0 => c.default_segment_interval,
        ms => Duration::from_millis(i64::from(ms)),
    };
    if interval < c.min_segment_interval || interval > c.max_segment_interval {
        return Err(DebugError::IntervalOutOfRange);
    }
    let segment_interval_ms =
        u32::try_from(interval.as_millis()).map_err(|_| DebugError::IntervalOutOfRange)?;
    if Duration::from_millis(i64::from(timeout_ms)) > c.max_timeout {
        return Err(DebugError::TimeoutOutOfRange);
    }
    let padding = padding_len(bitrate_kbps, segment_interval_ms);
    if segment_bound(padding) > MAX_SEGMENT_BYTES {
        return Err(DebugError::SegmentTooLarge);
    }
    Ok(Start {
        bitrate_kbps,
        segment_interval_ms,
        timeout_ms,
        padding,
    })
}

/// The `Service` listing of a mock stream with swarm URI `swarm_uri`.
pub(crate) fn mock_listing(swarm_uri: String) -> Listing {
    Listing {
        content: Some(Content::Service(Service {
            r#type: MOCK_STREAM_SERVICE.to_owned(),
            swarm_uri,
        })),
    }
}

/// A mock stream the node writes.
pub(crate) struct Producer {
    pub(crate) network_key: Vec<u8>,
    pub(crate) start: Start,
    pub(crate) started_at: Time,
    /// Its swarm's URI and transfer, and its listing's keeper, once its
    /// swarm is created.
    pub(crate) swarm_uri: String,
    pub(crate) transfer: Option<TransferId>,
    pub(crate) keeper: Option<ListingKeeper>,
    pub(crate) segments: u64,
    pub(crate) bytes_written: u64,
    pub(crate) stop: Rc<Stop>,
}

/// Why a mock stream's writes ended.
enum End {
    Stopped,
    TimedOut,
    Failed,
}

impl<K: Kv, H: Host, T: Timer> Shared<K, H, T> {
    /// Runs mock stream `id` under `key` from its swarm's creation to its
    /// end, answering `reply` with its id once the swarm is created; see
    /// the crate docs.
    pub(crate) async fn run_producer(self: Rc<Self>, id: u64, key: Key, reply: Rc<Reply<u64>>) {
        let Some((network_key, start, started_at, stop)) =
            self.producers.borrow().get(&id).map(|p| {
                (
                    p.network_key.clone(),
                    p.start.clone(),
                    p.started_at,
                    p.stop.clone(),
                )
            })
        else {
            return;
        };
        let options = mock_stream_options();
        let created = self
            .host
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key,
            })
            .await;
        let (swarm, swarm_id, writer) = match created {
            Ok(c) => c,
            Err(e) => {
                let e = self.host_failed("creating a mock stream swarm", &e);
                self.producers.borrow_mut().remove(&id);
                reply.set(Err(e));
                return;
            }
        };
        self.metrics.borrow_mut().mock_streams_started += 1;
        let swarm_uri = Uri::new(swarm_id.clone(), options.uri_options()).to_string();
        let t = self.transfers.add(swarm, &swarm_id, Vec::new());
        let keeper = ListingKeeper::new(network_key.clone(), mock_listing(swarm_uri.clone()));
        {
            let (run, directory) = (keeper.clone(), self.directory.clone());
            let config = self.config.keeper;
            self.spawn(async move { run.run(&directory, config, |_| {}).await }.boxed_local());
        }
        if let Some(p) = self.producers.borrow_mut().get_mut(&id) {
            p.swarm_uri = swarm_uri;
            p.transfer = Some(t);
            p.keeper = Some(keeper.clone());
        }
        self.publish_on_joined(t, &network_key);
        reply.set(Ok(id));

        let end = self
            .write_segments(id, &start, started_at, writer, &stop)
            .await;
        match end {
            End::Stopped => tracing::debug!(id, "mock stream stopped"),
            End::TimedOut => tracing::debug!(id, "mock stream timed out"),
            End::Failed => tracing::debug!(id, "mock stream write failed"),
        }
        keeper.stop();
        self.transfers.remove(t);
        self.remove_host_swarm(swarm).await;
        self.producers.borrow_mut().remove(&id);
        self.metrics.borrow_mut().mock_streams_ended += 1;
    }

    /// Writes stream `id`'s segments on their schedule until it ends.
    async fn write_segments<W: SwarmWrite>(
        &self,
        id: u64,
        start: &Start,
        started_at: Time,
        writer: W,
        stop: &Stop,
    ) -> End {
        let mut writer = match RecordWriter::new(writer, MOCK_FRAME, MAX_SEGMENT_BYTES) {
            Ok(w) => w,
            Err(e) => {
                self.metrics.borrow_mut().write_errors += 1;
                tracing::warn!(error = %e, "a mock stream's frame size was refused");
                return End::Failed;
            }
        };
        let interval = start.interval();
        let deadline = start.deadline(started_at);
        let mut segment = MockStreamSegment {
            id,
            timestamp: 0,
            padding: vec![0; start.padding],
        };
        let mut due = started_at;
        loop {
            let at = deadline.map_or(due, |d| due.min(d));
            if stop.until(self.timer.sleep_until(at)).await.is_none() {
                return End::Stopped;
            }
            let now = self.timer.now();
            if deadline.is_some_and(|d| now >= d) {
                return End::TimedOut;
            }
            if now < due {
                continue;
            }
            segment.timestamp = now.unix_nanos() / 1000;
            match stop.until(writer.write(&segment)).await {
                None => return End::Stopped,
                Some(Err(e)) => {
                    self.metrics.borrow_mut().write_errors += 1;
                    tracing::warn!(error = %e, "writing a mock stream failed");
                    return End::Failed;
                }
                Some(Ok(())) => {}
            }
            self.metrics.borrow_mut().segments_written += 1;
            if let Some(p) = self.producers.borrow_mut().get_mut(&id) {
                p.segments += 1;
                p.bytes_written += segment.encoded_len() as u64;
            }
            due = next_due(started_at, interval, now);
        }
    }
}

/// The first segment time after `now` of a stream that started at
/// `started_at` with segments every `interval`.
fn next_due(started_at: Time, interval: Duration, now: Time) -> Time {
    let step = interval.as_nanos().max(1);
    let elapsed = (now - started_at).as_nanos().max(0);
    let k = elapsed / step + 1;
    started_at + Duration::from_nanos(step.saturating_mul(k))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use strims_ppspp::SwarmId;

    use super::*;

    #[test]
    fn padding_scales_with_the_interval() {
        // 8 kbps is 1000 bytes a second, whatever the interval.
        assert_eq!(padding_len(8, 1000), 1000);
        assert_eq!(padding_len(8, 100), 100);
        assert_eq!(padding_len(8, 10_000), 10_000);
        assert_eq!(padding_len(0, 1000), 0);
        let c = Config::default();
        for ms in [100, 250, 1000, 4000] {
            let s = check_start(&c, 1000, ms, 0).unwrap();
            let per_second = s.padding as u64 * 1000 / u64::from(ms);
            assert_eq!(per_second, 125_000, "{ms} ms");
        }
    }

    #[test]
    fn requests_out_of_bounds_are_refused() {
        let c = Config::default();
        let s = check_start(&c, 1000, 0, 0).unwrap();
        assert_eq!((s.segment_interval_ms, s.timeout_ms), (1000, 0));
        assert_eq!(s.deadline(Time::EPOCH), None);
        assert_eq!(check_start(&c, 0, 100, 0).unwrap().padding, 0);
        assert_eq!(
            check_start(&c, 50_001, 100, 0),
            Err(DebugError::BitrateOutOfRange)
        );
        assert_eq!(
            check_start(&c, 1000, 99, 0),
            Err(DebugError::IntervalOutOfRange)
        );
        assert_eq!(
            check_start(&c, 100, 10_001, 0),
            Err(DebugError::IntervalOutOfRange)
        );
        assert!(check_start(&c, 100, 10_000, 0).is_ok());
        let day = 24 * 60 * 60 * 1000;
        let s = check_start(&c, 1000, 1000, day).unwrap();
        assert_eq!(
            s.deadline(Time::EPOCH),
            Some(Time::EPOCH + Duration::HOUR * 24)
        );
        assert_eq!(
            check_start(&c, 1000, 1000, day + 1),
            Err(DebugError::TimeoutOutOfRange)
        );
        // The highest bitrate fits only at short intervals.
        assert!(check_start(&c, 50_000, 100, 0).is_ok());
        assert_eq!(
            check_start(&c, 50_000, 1000, 0),
            Err(DebugError::SegmentTooLarge)
        );
    }

    #[test]
    fn segments_fit_the_record_bound() {
        let c = Config {
            max_bitrate_kbps: u32::MAX,
            ..Config::default()
        };
        for ms in [100, 1000, 10_000] {
            // The highest bitrate whose segments fit.
            let (mut ok, mut over) = (0u32, u32::MAX);
            while over - ok > 1 {
                let mid = ok + (over - ok) / 2;
                if check_start(&c, mid, ms, 0).is_ok() {
                    ok = mid;
                } else {
                    over = mid;
                }
            }
            let kbps = ok;
            let s = check_start(&c, kbps, ms, 0).unwrap();
            let worst = MockStreamSegment {
                id: u64::MAX,
                timestamp: i64::MIN,
                padding: vec![0; s.padding],
            };
            assert!(worst.encoded_len() <= MAX_SEGMENT_BYTES, "{ms} ms");
            assert_eq!(segment_bound(s.padding), worst.encoded_len());
            let over = check_start(&c, kbps + 1, ms, 0);
            assert_eq!(over, Err(DebugError::SegmentTooLarge), "{ms} ms");
        }
    }

    #[test]
    fn mock_stream_uris_carry_their_options() {
        let o = mock_stream_options();
        let uri = Uri::new(SwarmId::new(&[0; 32]), o.uri_options()).to_string();
        assert!(
            uri.ends_with("&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=32&x.sc=16"),
            "{uri}"
        );
        assert_eq!(o.live_window * o.chunk_size, crate::MOCK_WINDOW_BYTES);
        assert_eq!(o.chunk_size * o.chunks_per_signature, MOCK_FRAME);
        let parsed = Uri::parse(&uri).unwrap();
        let read = parsed
            .options
            .swarm_options()
            .assign_defaults(&SwarmOptions::default_options());
        assert_eq!(
            read,
            SwarmOptions {
                live_window: read.live_window,
                ..o
            }
        );
    }

    #[test]
    fn a_late_segment_skips_the_ticks_it_missed() {
        let t0 = Time::from_unix_millis(1_000);
        let s = Duration::SECOND;
        assert_eq!(next_due(t0, s, t0), t0 + s);
        assert_eq!(next_due(t0, s, t0 + Duration::from_millis(10)), t0 + s);
        assert_eq!(next_due(t0, s, t0 + s), t0 + s * 2);
        assert_eq!(
            next_due(t0, s, t0 + Duration::from_millis(3500)),
            t0 + s * 4
        );
    }
}
