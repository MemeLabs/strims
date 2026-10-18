// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live video between a Go and a Rust node over a direct link: the
//! helper's `video-*` verbs (a writer swarm as internal/videoingress
//! writes a channel's, and internal/videoegress's stream reader), and
//! `strims_video`'s control on a Rust node composed as in
//! [`directory`](crate::directory), beside its network, transfer and
//! directory controls, its RTMP ingress listening on loopback through a
//! `NetVideoHost`. Video swarms carry packed records, which both sides
//! must frame and read alike.
//!
//! - `rust_ingest_go_egress`: Rust owns the network and a channel on it,
//!   and Go joins. A synthetic stream of 1-second GOPs, published over
//!   RTMP to Rust's ingress, starts the channel's session; once its first
//!   records are written and Rust lists them, Go opens a stream of the
//!   ingest swarm, and the publisher goes on once the swarm runs with Go.
//!   Go's directory view shows the channel's media listing, of the
//!   records' MIME type and the ingest swarm's URI, with its live
//!   snippet, and Go's reader, joined mid stream, reads whole records
//!   that are a contiguous run of those the script produces.
//! - `go_writer_rust_egress`: Go owns the network, and Rust joins. Go
//!   writes the script's records, as Rust's segmenter makes them, to a
//!   swarm of the ingress options under a plain chunkstream writer, whose
//!   URI is the one Rust's ingress gives such a swarm. Rust opens a
//!   stream of it after the first batch, and Go writes the rest a record
//!   at a time; Rust's whole records are byte-identical to a contiguous
//!   run of them.
//!
//! Neither writer publishes its last partial integrity unit, so the
//! readers ask for records well before the end.

use std::path::{Path, PathBuf};
use std::time::Duration as StdDuration;

use prost::Message as _;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_crypto::{SignableMessage, key_from_seed};
use strims_dao::schema::network::network_key;
use strims_kv::MemoryKv;
use strims_node::app::{NodeHost, NodeTimer};
use strims_node::video_host::NetVideoHost;
use strims_node::{SwarmId, SystemClock, TokioNet};
use strims_ppspp::Uri;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::{Listing, ListingSnippet};
use strims_proto::network::v1::{CreateInvitationRequest, NetworkFrontendService};
use strims_proto::service::Context;
use strims_proto::video::v1::VideoIngressConfig;
use strims_transfer::TransferId;
use strims_video::amf0::Value;
use strims_video::hls::HlsRoutes;
use strims_video::synth::publisher::{Publisher, PublisherConfig};
use strims_video::synth::script::{
    MetaValue, Script, ScriptConfig, Tag, TagKind, expected_records,
};
use strims_video::{
    CutRule, DecoderConfig, EgressPiece, EgressStream, FlvSource, SourceEvent, SwarmKind,
    VideoControl, fmp4_mime, ingress_options,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::oneshot;
use tokio::task::{LocalSet, spawn_local};
use tokio::time::Instant;

use crate::directory::{
    RustMember, Timer, go_listings, go_options, go_until, id_key, rust_member, until,
};
use crate::harness::{GoEvent, GoNode, STEP, helper};

type Video = VideoControl<MemoryKv, NodeHost, NetVideoHost<TokioNet>, Timer>;
type Stream = EgressStream<MemoryKv, NodeHost, NetVideoHost<TokioNet>, Timer>;

/// The script time sent before Go's stream opens, in ms.
const BEFORE_GO_MS: u32 = 6_000;
/// How many times faster than its tag times the rest of the script goes.
const SPEED_UP: u64 = 2;
/// The whole records each reader reads.
const RECORDS: usize = 6;
/// The records Go writes before Rust's stream opens.
const FIRST_BATCH: usize = 4;
/// The time between the records Go writes after that.
const RECORD_INTERVAL: StdDuration = StdDuration::from_millis(500);
/// How long a reader may take for its records, and for each piece.
const READ_STEP: StdDuration = StdDuration::from_secs(120);
/// The key seed of Go's writer swarm.
const WRITER_SEED: [u8; 32] = [0x6e; 32];

/// 24 seconds of 1-second GOPs: 24 records of about 140 KiB, all within
/// either side's swarm window.
fn script() -> Script {
    Script::new(&ScriptConfig {
        gop_frames: 30,
        duration_secs: 24,
        ..ScriptConfig::default()
    })
}

/// The decoder configs of `script`, in init order.
fn script_configs(script: &Script) -> Vec<DecoderConfig> {
    let mut source = FlvSource::new();
    let mut configs = Vec::new();
    for tag in &script.tags {
        let e = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, Time::EPOCH),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, Time::EPOCH),
            TagKind::Metadata => continue,
        };
        if let Ok(Some(SourceEvent::Config(c))) = e {
            configs.push(c);
        }
    }
    configs
}

/// The URI of a swarm of the ingress options under the key `public`.
fn ingress_uri(public: &[u8]) -> String {
    Uri::new(SwarmId::new(public), ingress_options().uri_options()).to_string()
}

/// Where `got` starts in `expected`, which it is a contiguous run of.
fn run_of(expected: &[Vec<u8>], got: &[Vec<u8>]) -> usize {
    let start = expected
        .iter()
        .position(|r| *r == got[0])
        .expect("the first record is one of the expected");
    let end = start + got.len();
    assert!(
        end <= expected.len() && got == &expected[start..end],
        "records {start}..{end} of {} differ",
        expected.len()
    );
    start
}

/// A directory of record files for `test`, emptied.
fn record_dir(test: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join("node-interop-video")
        .join(test);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Writes `records` to `path` as the helper reads them: each a 4-byte
/// big-endian length and its bytes.
fn write_records(path: &Path, records: &[Vec<u8>]) {
    let mut b = Vec::new();
    for r in records {
        b.extend_from_slice(&u32::try_from(r.len()).unwrap().to_be_bytes());
        b.extend_from_slice(r);
    }
    std::fs::write(path, b).unwrap();
}

fn read_records(path: &Path) -> Vec<Vec<u8>> {
    let b = std::fs::read(path).unwrap();
    let mut records = Vec::new();
    let mut rest = b.as_slice();
    while !rest.is_empty() {
        let len = u32::from_be_bytes(rest[..4].try_into().unwrap()) as usize;
        records.push(rest[4..4 + len].to_vec());
        rest = &rest[4 + len..];
    }
    records
}

fn snippet() -> ListingSnippet {
    ListingSnippet {
        title: "synthetic bars".into(),
        description: "a test pattern".into(),
        channel_name: "rust's channel".into(),
        ..Default::default()
    }
}

/// A Rust member with a video control, its RTMP ingress on `TokioNet`.
struct RustVideo {
    m: RustMember,
    video: Video,
}

async fn rust_video(seed: u8) -> RustVideo {
    let m = rust_member(seed).await;
    let mut config = strims_video::Config::default();
    // The publisher sends nothing once the script is sent, while the test
    // waits on Go.
    config.ingress.media_idle = Duration::from_secs(300);
    let video = VideoControl::new(
        m.host.clone(),
        NetVideoHost(TokioNet),
        m.store.clone(),
        m.profile.clone(),
        m.control.clone(),
        m.transfers.clone(),
        m.directory.clone(),
        HlsRoutes::new(),
        NodeTimer(SystemClock),
        det_rng(u64::from(seed) + 5),
        config,
    );
    spawn_local({
        let video = video.clone();
        async move { video.run().await }
    });
    RustVideo { m, video }
}

impl RustVideo {
    /// The media listings of this side's view of network `network_id`.
    fn media(&self, network_id: u64) -> Vec<(Media, Option<ListingSnippet>)> {
        self.m
            .directory
            .listings(network_id)
            .into_iter()
            .filter_map(|l| match l.listing.content {
                Some(Content::Media(m)) => Some((m, l.snippet)),
                _ => None,
            })
            .collect()
    }

    /// Waits until the transfer `t` runs with the peer `go`.
    async fn runs_with(&self, what: &str, go: &GoNode, t: TransferId) {
        until(what, || {
            self.m.transfers.channels(go.host_id, t).map(|_| ())
        })
        .await;
    }
}

/// A free loopback address: the ingress refuses port 0.
fn free_addr() -> String {
    let l = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    l.local_addr().unwrap().to_string()
}

/// An RTMP connection to the ingress, and the publisher on it.
struct Publish {
    stream: TcpStream,
    publisher: Publisher,
    metadata: Vec<(String, MetaValue)>,
    input: Vec<u8>,
    output: Vec<u8>,
    /// The server closed the connection.
    closed: bool,
}

impl Publish {
    /// Publishes to `live/<stream key>` at `addr`, once the server
    /// accepts it.
    async fn connect(addr: &str, stream_key: &str, script: &Script) -> Publish {
        let stream = TcpStream::connect(addr).await.unwrap();
        stream.set_nodelay(true).unwrap();
        let mut p = Publish {
            stream,
            publisher: Publisher::new(PublisherConfig::new("live", stream_key), &mut det_rng(0x70)),
            metadata: script.metadata.clone(),
            input: vec![0; 64 * 1024],
            output: vec![0; 64 * 1024],
            closed: false,
        };
        p.until("the publish's answer", |p| {
            p.publisher.publish_accepted() || p.publisher.refused().is_some()
        })
        .await;
        assert!(
            p.publisher.publish_accepted(),
            "{:?}",
            p.publisher.refused()
        );
        p
    }

    /// Writes what the publisher queued.
    async fn flush(&mut self) {
        loop {
            let n = self.publisher.poll_transmit(&mut self.output);
            if n == 0 {
                return;
            }
            self.stream.write_all(&self.output[..n]).await.unwrap();
        }
    }

    /// Takes and answers the server's bytes until `deadline`, or until the
    /// server closes the connection.
    async fn exchange(&mut self, deadline: Instant) {
        while !self.closed {
            self.flush().await;
            let read = tokio::select! {
                biased;
                r = self.stream.read(&mut self.input) => Some(r),
                () = tokio::time::sleep_until(deadline) => None,
            };
            match read {
                None => return,
                Some(Ok(0) | Err(_)) => self.closed = true,
                Some(Ok(n)) => self.publisher.handle_input(&self.input[..n]).unwrap(),
            }
        }
    }

    /// Exchanges with the server until `done` holds.
    async fn until(&mut self, what: &str, done: impl Fn(&Publish) -> bool) {
        let end = Instant::now() + STEP;
        while !done(self) {
            let now = Instant::now();
            assert!(now < end && !self.closed, "{what} didn't happen");
            self.exchange(now + StdDuration::from_millis(10)).await;
        }
    }

    /// Exchanges with the server until `go` is sent.
    async fn wait(&mut self, mut go: oneshot::Receiver<()>) {
        while go.try_recv().is_err() {
            assert!(!self.closed, "the server closed the connection");
            self.exchange(Instant::now() + StdDuration::from_millis(50))
                .await;
        }
    }

    fn send_tag(&mut self, tag: &Tag) {
        match tag.kind {
            TagKind::Video => self.publisher.send_video(tag.timestamp, &tag.body).unwrap(),
            TagKind::Audio => self.publisher.send_audio(tag.timestamp, &tag.body).unwrap(),
            TagKind::Metadata => {
                let fields: Vec<(&str, Value<'_>)> = self
                    .metadata
                    .iter()
                    .map(|(k, v)| {
                        let v = match v {
                            MetaValue::Number(n) => Value::Number(*n),
                            MetaValue::Bool(b) => Value::Boolean(*b),
                            MetaValue::String(s) => Value::String(s),
                        };
                        (k.as_str(), v)
                    })
                    .collect();
                self.publisher.send_metadata(&fields).unwrap();
            }
        }
    }

    /// Sends `tags`, each `speed_up` times faster than its time after the
    /// first, or as fast as the connection takes them.
    async fn send(&mut self, tags: &[Tag], speed_up: Option<u64>) {
        let start = Instant::now();
        let first = tags.first().map_or(0, |t| t.timestamp);
        for tag in tags {
            let due = speed_up.map_or(start, |s| {
                start + StdDuration::from_millis(u64::from(tag.timestamp - first) / s)
            });
            self.exchange(due).await;
            assert!(!self.closed, "the server closed the connection");
            self.send_tag(tag);
        }
        self.flush().await;
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn rust_ingest_go_egress() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        420,
        LocalSet::new().run_until(async {
            let began = Instant::now();
            let dir = record_dir("rust_ingest_go_egress");
            let script = script();
            let expected = expected_records(&script, CutRule::default());
            let mime = fmp4_mime(&script_configs(&script));
            assert_eq!(mime, "video/mp4;codecs=\"avc1.64001f,mp4a.40.2\"");

            let r = rust_video(0x5d).await;
            let owned = r.m.create("video", None).await;
            let net_id = owned.id;
            let key = network_key(&owned);
            let key_hex = hex::encode(&key);
            until("rust to serve", || {
                r.m.directory.networks().first()?.serving.then_some(())
            })
            .await;
            let invitation =
                r.m.frontend
                    .create_invitation(
                        &Context::default(),
                        CreateInvitationRequest {
                            network_id: net_id,
                            bootstrap_client_id: 0,
                        },
                    )
                    .await
                    .unwrap()
                    .invitation
                    .unwrap();
            let mut go = GoNode::spawn(&bin, "gv0", go_options(0x5c)).await;
            let (go_net, joined_key) = id_key(
                &go.cmd(&format!(
                    "net-join {}",
                    hex::encode(invitation.encode_to_vec())
                ))
                .await
                .unwrap(),
            );
            assert_eq!(joined_key, key_hex);
            assert_eq!(r.m.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            until("rust to bind go", || r.m.is_bound(go.host_id).then_some(())).await;

            // a channel, and the ingress listening for it
            let channel = r.video.create_channel(&key, snippet()).await.unwrap();
            let channel_key = channel.key.as_ref().unwrap().public.clone();
            let addr = free_addr();
            r.video
                .set_ingress_config(VideoIngressConfig {
                    enabled: true,
                    server_addr: addr.clone(),
                    ..Default::default()
                })
                .await
                .unwrap();
            until("the ingress to listen", || {
                (r.video.metrics().ingress.listens > 0).then_some(())
            })
            .await;
            let url = r.video.channel_url(channel.id).await.unwrap();
            assert_eq!(url.server_addr, addr);
            let uri = ingress_uri(&channel_key);
            let t = TransferId::new(&SwarmId::new(&channel_key), b"");

            // the publisher sends the script's start, waits for Go's
            // stream, then sends the rest
            let split = script
                .tags
                .iter()
                .position(|t| t.timestamp >= BEFORE_GO_MS)
                .unwrap();
            let (sent_tx, sent_rx) = oneshot::channel();
            let (go_tx, go_rx) = oneshot::channel();
            let (end_tx, end_rx) = oneshot::channel();
            let publish = spawn_local({
                let script = script.clone();
                let stream_key = url.stream_key.clone();
                async move {
                    let mut p = Publish::connect(&addr, &stream_key, &script).await;
                    p.send(&script.tags[..split], None).await;
                    sent_tx.send(()).unwrap();
                    p.wait(go_rx).await;
                    p.send(&script.tags[split..], Some(SPEED_UP)).await;
                    p.wait(end_rx).await;
                    p.publisher.unpublish().unwrap();
                    p.until("the server to close", |p| p.closed).await;
                }
            });
            sent_rx
                .await
                .expect("the publisher sent the script's start");
            until("the ingest's first records", || {
                (r.video.metrics().ingress.segments >= 3).then_some(())
            })
            .await;
            let media = until(
                "the channel's listing and live snippet in rust's view",
                || {
                    r.media(net_id)
                        .into_iter()
                        .find_map(|(m, s)| s.is_some_and(|s| s.live).then_some(m))
                },
            )
            .await;
            assert_eq!(
                (media.mime_type.as_str(), media.swarm_uri.as_str()),
                (mime.as_str(), uri.as_str())
            );
            let swarms = r.video.video_swarms();
            assert_eq!(swarms.len(), 1);
            assert_eq!(
                (swarms[0].kind, swarms[0].transfer_id, &swarms[0].swarm_uri),
                (SwarmKind::Ingest, t, &uri)
            );

            // go's stream of the ingest swarm, joined mid stream
            let out = dir.join("go-records");
            let opened = go
                .cmd(&format!(
                    "video-read {uri} {key_hex} {RECORDS} {}",
                    out.display()
                ))
                .await
                .unwrap();
            assert_eq!(opened, hex::encode(t.as_bytes()));
            r.runs_with("the ingest swarm to run with go", &go, t).await;
            go_tx.send(()).unwrap();

            // the channel's listing and live snippet in go's view
            let listing = Listing {
                content: Some(Content::Media(media.clone())),
            };
            go_until(&mut go, go_net, "the channel's listing", |ls| {
                ls.iter().any(|l| {
                    l.listing.as_ref() == Some(&listing)
                        && l.snippet.as_ref().is_some_and(|s| {
                            s.live && s.key == channel_key && s.title == "synthetic bars"
                        })
                })
            })
            .await;
            let items = go_listings(&mut go, go_net).await;
            let shown = items
                .iter()
                .find_map(|l| (l.listing.as_ref() == Some(&listing)).then_some(l.snippet.clone()))
                .flatten()
                .unwrap();
            shown
                .verify_message()
                .expect("the channel key signed the snippet");
            assert_eq!(shown.channel_name, "rust's channel");

            let path = out.display().to_string();
            let GoEvent::VideoRead(_, read) = go
                .wait_until(
                    "go's records",
                    Instant::now() + READ_STEP,
                    |e| matches!(e, GoEvent::VideoRead(f, _) if *f == path),
                )
                .await
            else {
                unreachable!()
            };
            assert_eq!(read, Ok(0), "go's read");
            let got = read_records(&out);
            assert_eq!(got.len(), RECORDS);
            let start = run_of(&expected, &got);
            let sizes: Vec<usize> = got.iter().map(Vec::len).collect();
            eprintln!(
                "go read records {start}..{} of {}: {sizes:?} bytes",
                start + got.len(),
                expected.len()
            );

            // the publisher's unpublish ends the session
            end_tx.send(()).unwrap();
            publish.await.unwrap();
            until("the session to end", || {
                (r.video.ingress_streams().is_empty() && r.video.video_swarms().is_empty())
                    .then_some(())
            })
            .await;
            let m = r.video.metrics().ingress;
            eprintln!("rust's ingress: {m:?}");
            assert_eq!((m.connections, m.sessions, m.refused), (1, 1, 0));
            assert_eq!(m.segments, expected.len() as u64);
            assert_eq!((m.dropped_units, m.media_errors, m.write_errors), (0, 0, 0));
            eprintln!("rust_ingest_go_egress took {:?}", began.elapsed());
        }),
    )
    .await;
}

/// Has Go write `records`, as the next of its swarm's, and checks its
/// totals: `written` records before them.
async fn go_write(go: &mut GoNode, dir: &Path, written: usize, records: &[Vec<u8>]) {
    let path = dir.join(format!("records-{written}"));
    write_records(&path, records);
    let reply = go
        .cmd(&format!("video-write {}", path.display()))
        .await
        .unwrap();
    let (n, _) = reply.split_once(' ').unwrap();
    assert_eq!(n.parse::<usize>().unwrap(), written + records.len());
}

/// The next `n` whole records of `stream`.
async fn stream_records(stream: &mut Stream, n: usize) -> Vec<Vec<u8>> {
    let mut got = Vec::new();
    let mut record = Vec::new();
    while got.len() < n {
        let piece = tokio::time::timeout(READ_STEP, stream.next(usize::MAX))
            .await
            .expect("a piece in time")
            .expect("a piece");
        match piece {
            EgressPiece::Data(d) => record.extend_from_slice(d),
            EgressPiece::RecordEnd => got.push(std::mem::take(&mut record)),
            EgressPiece::Discontinuity => panic!("a discontinuity after {} records", got.len()),
        }
    }
    got
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn go_writer_rust_egress() {
    let Some(bin) = helper() else {
        return;
    };
    crate::bounded(
        420,
        LocalSet::new().run_until(async {
            let began = Instant::now();
            let dir = record_dir("go_writer_rust_egress");
            let expected = expected_records(&script(), CutRule::default());
            assert!(expected.len() >= FIRST_BATCH + RECORDS + 4);

            let mut o = go_options(0x6c);
            o.directory = false;
            let mut go = GoNode::spawn(&bin, "gv1", o).await;
            let (go_net, key_hex) = id_key(
                &go.cmd(&format!("net-create {}", hex::encode("video")))
                    .await
                    .unwrap(),
            );
            let key = hex::decode(&key_hex).unwrap();
            let invitation = go.cmd(&format!("net-invite {go_net}")).await.unwrap();
            let r = rust_video(0x6d).await;
            let joined = r.m.join(hex::decode(&invitation).unwrap()).await;
            assert_eq!(network_key(&joined), key);
            assert_eq!(r.m.node.dial(&go.tcp_uri).await.unwrap(), go.host_id);
            until("rust to bind go and renew", || {
                (r.m.is_bound(go.host_id) && r.m.trusted(joined.id)).then_some(())
            })
            .await;

            // go's writer swarm, the URI of an ingest swarm under its key
            let writer = key_from_seed(&WRITER_SEED);
            let uri = go
                .cmd(&format!(
                    "video-write-open {} {key_hex}",
                    hex::encode(WRITER_SEED)
                ))
                .await
                .unwrap();
            assert_eq!(uri, ingress_uri(&writer.public));
            go_write(&mut go, &dir, 0, &expected[..FIRST_BATCH]).await;

            // rust's stream, then the rest a record at a time
            let mut stream = r
                .video
                .open_stream(&uri, std::slice::from_ref(&key))
                .await
                .unwrap();
            let t = stream.transfer_id();
            assert_eq!(t, TransferId::new(&SwarmId::new(&writer.public), b""));
            let swarms = r.video.video_swarms();
            assert_eq!(swarms.len(), 1);
            assert_eq!((swarms[0].kind, swarms[0].streams), (SwarmKind::Egress, 1));
            r.runs_with("rust's swarm to run with go", &go, t).await;
            let read = spawn_local(async move {
                let got = stream_records(&mut stream, RECORDS).await;
                (stream, got)
            });
            let mut written = FIRST_BATCH;
            while written < expected.len() && !read.is_finished() {
                tokio::time::sleep(RECORD_INTERVAL).await;
                go_write(&mut go, &dir, written, &expected[written..written + 1]).await;
                written += 1;
            }
            let (stream, got) = read.await.unwrap();
            let start = run_of(&expected, &got);
            let sizes: Vec<usize> = got.iter().map(Vec::len).collect();
            eprintln!(
                "rust read records {start}..{} of {} ({written} written): {sizes:?} bytes; {:?}",
                start + got.len(),
                expected.len(),
                stream.metrics()
            );
            drop(stream);
            // Go's writer frames every record as Rust's segmenter cut it:
            // none ends second in its frame
            go_write(&mut go, &dir, written, &expected[written..]).await;
            until("rust's swarm to go", || {
                r.video.video_swarms().is_empty().then_some(())
            })
            .await;
            eprintln!("go_writer_rust_egress took {:?}", began.elapsed());
        }),
    )
    .await;
}
