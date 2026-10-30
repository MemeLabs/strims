// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The RTMP ingress on fake nodes (see `strims_testworld`), with a fake
//! listener (`support`): a published stream's records, listing and
//! snippet on another node, refusals, replacement, channel changes, the
//! listener's lifecycle, deadlines, caps and `ListStreams`; and the signing
//! a unit-format session's swarm takes from its stream's metadata and
//! tracks.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

mod support;

use std::time::Duration;

use futures_util::StreamExt;
use strims_base::time::Time;
use strims_dao::schema::video::{VideoChannels, format_stream_key, new_channel};
use strims_dao::{Recv, Subscription, Table};
use strims_kv::MemoryKv;
use strims_ppspp::{SwarmId, Uri};
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::types::Key;
use strims_proto::video::v1::video_channel::{Local, Owner};
use strims_proto::video::v1::{
    VideoChannel, VideoIngressClient, VideoIngressConfig, VideoIngressListStreamsRequest,
};
use strims_rpc::{Router, Timer};
use strims_testworld::*;
use strims_transfer::TransferId;
use strims_video::hls::HlsRoutes;
use strims_video::ingress::unit_ingress_options;
use strims_video::synth::script::{
    Script, ScriptConfig, Tag, TagKind, expected_records, without_publish_times,
};
use strims_video::units::{ReaderEvent, UnitReader, UnitSigning};
use strims_video::{
    AccessUnit, Config, CutRule, DecoderConfig, EgressPiece, EgressStream, FlvSource, IngestFormat,
    IngressSettings, NoVideoHost, SourceEvent, StreamFormat, SwarmKind, TrackId, VideoControl,
    VideoEvent, VideoHost, VideoSwarm, bind_frontend, fmp4_mime, units_mime,
};
use support::{FakeVideo, TestPublisher};

type Control<V> = VideoControl<MemoryKv, FakeHost, V, TokioTimer>;
type Stream<V> = EgressStream<MemoryKv, FakeHost, V, TokioTimer>;

const ADDR: &str = "127.0.0.1:1935";
const BAD_NAME: &str = "NetStream.Publish.BadName";
const FAILED: &str = "NetStream.Publish.Failed";
const HEVC: &str = "HEVC (hvc1) is not supported; send H.264 with AAC";
const REPLACED: &str = "replaced by a new publish to the channel";
/// The ingest swarm's chunk, and its integrity unit of 8 chunks.
const CHUNK: u64 = 4096;
const LARGE_UNIT: u64 = 8 * CHUNK;

fn start<V: VideoHost>(n: &Node, video: V, config: Config) -> Control<V> {
    let control = VideoControl::new(
        n.host.clone(),
        video,
        n.store.clone(),
        n.profile.clone(),
        n.network.clone(),
        n.transfers.clone(),
        n.directory.clone(),
        HlsRoutes::new(),
        n.timer.clone(),
        n.rng(),
        config,
    );
    let c = control.clone();
    n.spawn(async move { c.run().await });
    control
}

fn enabled(addr: &str) -> VideoIngressConfig {
    VideoIngressConfig {
        enabled: true,
        server_addr: addr.into(),
        ..Default::default()
    }
}

fn snippet(title: &str) -> ListingSnippet {
    ListingSnippet {
        title: title.into(),
        description: "desc".into(),
        channel_name: "chan".into(),
        ..Default::default()
    }
}

fn channel_key(c: &VideoChannel) -> &Key {
    c.key.as_ref().unwrap()
}

fn swarm_id(c: &VideoChannel) -> SwarmId {
    SwarmId::new(&channel_key(c).public)
}

/// The URI of a channel's swarm.
fn channel_uri(c: &VideoChannel) -> String {
    format!(
        "magnet:?xt=urn:ppspp:{}&x.im=2&x.hf=7&x.sa=2&x.cs=4096&x.ps=8&x.sc=16",
        swarm_id(c)
    )
}

/// The decoder configs of `script`, in init order.
fn script_configs(script: &Script) -> Vec<DecoderConfig> {
    let mut source = FlvSource::new();
    let mut configs = Vec::new();
    let at = Time::from_unix_millis(0);
    for tag in &script.tags {
        let e = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, at),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, at),
            TagKind::Metadata => continue,
        };
        if let Ok(Some(SourceEvent::Config(c))) = e {
            configs.push(c);
        }
    }
    configs
}

/// The media listings of network `network_id`'s view on `n`: id, content
/// and snippet.
fn media(n: &Node, network_id: u64) -> Vec<(u64, Media, Option<ListingSnippet>)> {
    n.directory
        .listings(network_id)
        .into_iter()
        .filter_map(|l| match l.listing.content {
            Some(Content::Media(m)) => Some((l.id, m, l.snippet)),
            _ => None,
        })
        .collect()
}

/// The snippet of the first media listing of network 10 on `n`.
fn shown_snippet(n: &Node) -> Option<ListingSnippet> {
    media(n, 10).first().and_then(|(_, _, s)| s.clone())
}

/// The next event `f` picks within 60 seconds.
async fn event(
    events: &mut Subscription<VideoEvent>,
    what: &str,
    f: impl Fn(&VideoEvent) -> bool,
) -> VideoEvent {
    let found = tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            match events.next().await {
                Some(Recv::Event(e)) if f(&e) => return (*e).clone(),
                Some(_) => {}
                None => panic!("the events ended"),
            }
        }
    })
    .await;
    found.unwrap_or_else(|_| panic!("timed out waiting for {what}"))
}

/// The next session end.
async fn ended(events: &mut Subscription<VideoEvent>) -> (u64, String) {
    match event(events, "a stream's end", |e| {
        matches!(e, VideoEvent::IngressEnded { .. })
    })
    .await
    {
        VideoEvent::IngressEnded { channel_id, reason } => (channel_id, reason),
        _ => unreachable!(),
    }
}

/// The records of `script`, as [`read`] reads them: without their units'
/// publish times, which are the ingest's own.
fn script_records(script: &Script) -> Vec<Vec<u8>> {
    expected_records(script, CutRule::default())
        .iter()
        .map(|r| without_publish_times(r))
        .collect()
}

/// The next `n` records of a stream without their publish times
/// ([`without_publish_times`]), each with whether a discontinuity came
/// before it.
async fn read<V: VideoHost>(s: &mut Stream<V>, n: usize) -> Vec<(Vec<u8>, bool)> {
    let mut out = Vec::new();
    let mut rec = Vec::new();
    let mut flagged = false;
    while out.len() < n {
        let piece = tokio::time::timeout(Duration::from_secs(10), s.next(usize::MAX))
            .await
            .expect("a piece in time")
            .expect("a piece");
        match piece {
            EgressPiece::Data(d) => rec.extend_from_slice(d),
            EgressPiece::RecordEnd => {
                let r = without_publish_times(&std::mem::take(&mut rec));
                out.push((r, std::mem::take(&mut flagged)));
            }
            EgressPiece::Discontinuity => {
                rec.clear();
                flagged = true;
            }
        }
    }
    out
}

/// Records of a stream until one equals `last`, none after a
/// discontinuity.
async fn read_until<V: VideoHost>(s: &mut Stream<V>, last: &[u8], most: usize) -> Vec<Vec<u8>> {
    let mut got = Vec::new();
    while got.last().map(Vec::as_slice) != Some(last) {
        assert!(got.len() < most, "more records than expected");
        let (r, flagged) = read(s, 1).await.remove(0);
        assert!(!flagged, "a discontinuity");
        got.push(r);
    }
    got
}

/// Where `got` starts in `expected`, which it is a contiguous run of.
fn run_of(got: &[Vec<u8>], expected: &[Vec<u8>]) -> usize {
    assert!(!got.is_empty());
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

/// A node with network 10, a channel on it, and the ingress listening.
struct Ingest {
    world: World,
    a: Node,
    b: Option<Node>,
    net: Key,
    video: FakeVideo,
    va: Control<FakeVideo>,
    channel: VideoChannel,
    key: String,
    _pipes: Vec<Pipe>,
}

impl Ingest {
    async fn new(with_viewer: bool, config: Config) -> Ingest {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let b = if with_viewer {
            Some(node_in(&world, 2).await)
        } else {
            None
        };
        let members: Vec<&Node> = b.iter().collect();
        let (net, pipes) = owned_network(&a, &members, 10).await;
        let video = FakeVideo::default();
        let va = start(&a, video.clone(), config);
        let channel = va
            .create_channel(&net.public, snippet("first"))
            .await
            .unwrap();
        va.set_ingress_config(enabled(ADDR)).await.unwrap();
        wait_for(10, "the listener", || {
            video.listening().as_deref() == Some(ADDR)
        })
        .await;
        let key = va.channel_url(channel.id).await.unwrap().stream_key;
        Ingest {
            world,
            a,
            b,
            net,
            video,
            va,
            channel,
            key,
            _pipes: pipes,
        }
    }

    /// A publish of the channel from `peer`, answered.
    async fn publish(&self, peer: &str) -> TestPublisher {
        let mut p = TestPublisher::connect(&self.video, peer, "live", &self.key, &mut self.a.rng());
        p.answered().await;
        assert!(
            p.publisher.publish_accepted(),
            "{:?}",
            p.publisher.refused()
        );
        p
    }

    /// A publish of `app` and `key` from `peer`: its refusal.
    async fn refused(&self, peer: &str, app: &str, key: &str) -> (String, String) {
        let mut p = TestPublisher::connect(&self.video, peer, app, key, &mut self.a.rng());
        p.refusal().await
    }

    fn transfer_id(&self) -> TransferId {
        TransferId::new(&swarm_id(&self.channel), b"")
    }

    /// The bytes written into the channel's swarm, in all its sessions.
    fn written(&self) -> u64 {
        self.world.written(&swarm_id(&self.channel), "")
    }

    /// The node's ingest swarm, once added.
    fn ingest_swarm(&self) -> Option<VideoSwarm> {
        self.va
            .video_swarms()
            .into_iter()
            .find(|w| w.kind == SwarmKind::Ingest && w.added)
    }
}

fn refusal(code: &str, description: &str) -> (String, String) {
    (code.to_owned(), description.to_owned())
}

/// A publisher's stream reaches another node whole: the node finds its
/// listing, of the records' MIME type and the channel swarm's URI, with
/// the channel's live snippet, and reads a contiguous run of the records,
/// up to the last one, which the unpublish writes. The end takes the
/// listing, transfer and swarm down.
#[test]
fn a_published_stream_is_listed_and_read_by_another_node() {
    run(async {
        let s = Ingest::new(true, fmp4_config()).await;
        let b = s.b.as_ref().unwrap();
        let vb = start(b, NoVideoHost, Config::default());
        let mut events = s.va.subscribe();
        let before = s.a.timer.now().unix();
        let mut p = s.publish("10.0.0.1:50000").await;
        assert_eq!(
            event(&mut events, "the start", |e| matches!(
                e,
                VideoEvent::IngressStarted { .. }
            ))
            .await,
            VideoEvent::IngressStarted {
                channel_id: s.channel.id
            }
        );
        let t = s.transfer_id();
        assert!(s.a.transfers.is_published(t, s.a.net(&s.net.public)));
        let swarms = s.va.video_swarms();
        assert_eq!(swarms.len(), 1);
        assert_eq!(
            (swarms[0].kind, swarms[0].transfer_id, &swarms[0].swarm_uri),
            (SwarmKind::Ingest, t, &channel_uri(&s.channel))
        );

        let script = Script::new(&ScriptConfig::default());
        let expected = script_records(&script);
        assert!(expected.len() >= 4);
        let third = script.tags.len() / 3;
        p.send_tags(&script.tags[..third], &script).await;

        wait_for(10, "the listing and snippet", || shown_snippet(b).is_some()).await;
        let (_, m, shown) = media(b, 10).remove(0);
        assert_eq!(m.swarm_uri, channel_uri(&s.channel));
        assert_eq!(m.mime_type, fmp4_mime(&script_configs(&script)));
        assert_eq!(m.mime_type, "video/mp4;codecs=\"avc1.64001f,mp4a.40.2\"");
        let shown = shown.unwrap();
        // the server shows only snippets the swarm's key, the channel's,
        // signed
        let server =
            s.a.directory
                .metrics(&s.net.public)
                .unwrap()
                .server
                .unwrap();
        assert_eq!(server.snippets_unverified, 0, "{server:?}");
        assert_eq!(
            (
                shown.title.as_str(),
                shown.channel_name.as_str(),
                shown.live
            ),
            ("first", "chan", true)
        );
        let streams = s.va.ingress_streams();
        assert_eq!(streams.len(), 1);
        assert_eq!(shown.start_time, streams[0].created_at);
        assert!(shown.start_time >= before && shown.start_time <= s.a.timer.now().unix());

        let mut stream = vb
            .open_stream(
                &m.swarm_uri,
                std::slice::from_ref(&s.net.public),
                StreamFormat::from_mime(&m.mime_type),
            )
            .await
            .unwrap();
        p.send_tags(&script.tags[third..], &script).await;
        p.unpublish().await;
        let got = read_until(&mut stream, expected.last().unwrap(), expected.len()).await;
        run_of(&got, &expected);
        assert!(got.len() >= 3);

        assert_eq!(
            ended(&mut events).await,
            (s.channel.id, "the publisher ended the stream".to_owned())
        );
        p.until("the connection to close", |p| p.closed).await;
        assert!(s.va.ingress_streams().is_empty());
        assert!(s.va.video_swarms().is_empty());
        assert!(!s.a.transfers.list().contains(&t));
        assert_eq!(s.a.host.swarms_of(&swarm_id(&s.channel)), (0, 0));
        wait_for(20, "the listing to go", || media(b, 10).is_empty()).await;
        let m = s.va.metrics().ingress;
        assert_eq!((m.sessions, m.refused, m.connections), (1, 0, 1));
        assert_eq!(m.segments, expected.len() as u64);
        assert_eq!(
            m.bytes,
            expected.iter().map(|r| r.len() as u64).sum::<u64>()
        );
        assert_eq!((m.dropped_units, m.media_errors, m.write_errors), (0, 0, 0));
        assert_eq!(m.metadata, 1);
    });
}

/// Publishes of another app, a key that doesn't parse or names no
/// channel, a channel whose network the node hasn't joined, or past the
/// session cap are refused, each with its code and reason.
#[test]
fn publishes_are_refused_with_their_reasons() {
    run(async {
        let config = Config {
            ingress: IngressSettings {
                max_sessions: 1,
                format: IngestFormat::Fmp4,
                ..IngressSettings::default()
            },
            ..Config::default()
        };
        let s = Ingest::new(false, config).await;
        let profile_key = s.a.profile_key().clone();
        assert_eq!(
            s.refused("10.0.0.1:1", "app", &s.key).await,
            refusal(BAD_NAME, "the app must be \"live\"")
        );
        assert_eq!(
            s.refused("10.0.0.2:1", "live", "not a key!").await,
            refusal(BAD_NAME, "invalid stream key")
        );
        let unknown =
            format_stream_key(s.channel.id + 1000, &s.channel.token, &profile_key).unwrap();
        assert_eq!(
            s.refused("10.0.0.3:1", "live", &unknown).await,
            refusal(BAD_NAME, "unknown stream key")
        );
        let forged = format_stream_key(s.channel.id, b"another token", &profile_key).unwrap();
        assert_eq!(
            s.refused("10.0.0.4:1", "live", &forged).await,
            refusal(BAD_NAME, "unknown stream key")
        );

        let mut tx = s.a.store.write(s.a.timer.now()).await.unwrap();
        let owner = Owner::Local(Local {
            auth_key: profile_key.public.clone(),
            network_key: vec![9; 32],
        });
        let elsewhere = new_channel(&mut tx, owner, snippet("elsewhere"))
            .await
            .unwrap();
        let elsewhere = VideoChannels::insert(&mut tx, elsewhere).await.unwrap();
        tx.commit().await.unwrap();
        let key = s.va.channel_url(elsewhere.id).await.unwrap().stream_key;
        assert_eq!(
            s.refused("10.0.0.5:1", "live", &key).await,
            refusal(FAILED, "the channel's network is not joined")
        );

        let other =
            s.va.create_channel(&s.net.public, snippet("other"))
                .await
                .unwrap();
        let other_key = s.va.channel_url(other.id).await.unwrap().stream_key;
        let _live = s.publish("10.0.0.6:1").await;
        assert_eq!(
            s.refused("10.0.0.7:1", "live", &other_key).await,
            refusal(FAILED, "too many streams")
        );
        assert_eq!(s.va.metrics().ingress.refused, 6);
        assert_eq!(s.va.ingress_streams().len(), 1);
        assert_eq!(s.va.video_swarms().len(), 1);
    });
}

/// A stream of a codec the ingress doesn't take ends with the source's
/// message, which the publisher gets too.
#[test]
fn an_hevc_stream_ends_with_the_unsupported_message() {
    run(async {
        let s = Ingest::new(false, Config::default()).await;
        let mut events = s.va.subscribe();
        let mut p = s.publish("10.0.0.1:1").await;
        // An Enhanced RTMP keyframe sequence start of hvc1.
        let mut body = vec![0x90];
        body.extend_from_slice(b"hvc1");
        body.extend_from_slice(&[1, 2, 3, 4]);
        p.publisher.send_video(0, &body).unwrap();
        p.flush().await;
        assert_eq!(p.refusal().await, refusal(FAILED, HEVC));
        assert_eq!(ended(&mut events).await, (s.channel.id, HEVC.to_owned()));
        assert_eq!(s.va.metrics().ingress.media_errors, 1);
        assert!(s.va.video_swarms().is_empty());
    });
}

/// A stream without a keyframe within the record bound ends with the
/// segmenter's reason.
#[test]
fn a_record_past_its_bound_ends_the_session() {
    run(async {
        let config = Config {
            ingress: IngressSettings {
                max_record_bytes: 256 * 1024,
                format: IngestFormat::Fmp4,
                ..IngressSettings::default()
            },
            ..Config::default()
        };
        let s = Ingest::new(false, config).await;
        let mut events = s.va.subscribe();
        let mut p = s.publish("10.0.0.1:1").await;
        let script = Script::new(&ScriptConfig {
            gop_frames: 300,
            ..ScriptConfig::default()
        });
        p.send_tags(&script.tags, &script).await;
        let reason = "no keyframe arrived within the 262144-byte record bound";
        assert_eq!(ended(&mut events).await, (s.channel.id, reason.to_owned()));
        assert_eq!(p.refusal().await, refusal(FAILED, reason));
        assert_eq!(s.va.metrics().ingress.media_errors, 1);
    });
}

/// A second publish of a channel replaces its session: the first
/// connection closes, and a stream of the channel on the same node goes
/// on after one discontinuity with the new session's records.
#[test]
fn a_second_publish_replaces_the_first() {
    run(async {
        let s = Ingest::new(false, fmp4_config()).await;
        let mut events = s.va.subscribe();
        let mut first = s.publish("10.0.0.1:1").await;
        let mut stream =
            s.va.open_stream(
                &channel_uri(&s.channel),
                std::slice::from_ref(&s.net.public),
                StreamFormat::Packed,
            )
            .await
            .unwrap();
        let swarms = s.va.video_swarms();
        assert_eq!((swarms.len(), swarms[0].streams), (1, 1));

        let old = Script::new(&ScriptConfig {
            duration_secs: 6,
            ..ScriptConfig::default()
        });
        let old_records = script_records(&old);
        first.send_tags(&old.tags, &old).await;
        let before: Vec<Vec<u8>> = read(&mut stream, 2)
            .await
            .into_iter()
            .map(|(r, flagged)| {
                assert!(!flagged);
                r
            })
            .collect();
        run_of(&before, &old_records);

        let mut second = s.publish("10.0.0.1:2").await;
        assert_eq!(
            first.refusal().await,
            refusal(FAILED, "replaced by a new publish to the channel")
        );
        assert_eq!(
            ended(&mut events).await,
            (
                s.channel.id,
                "replaced by a new publish to the channel".to_owned()
            )
        );
        let new = Script::new(&ScriptConfig {
            video_bitrate: 800_000,
            ..ScriptConfig::default()
        });
        let new_records = script_records(&new);
        second.send_tags(&new.tags, &new).await;

        // Records of the first session may come before the discontinuity.
        let mut after = Vec::new();
        let mut flagged = false;
        while !flagged {
            let (r, f) = read(&mut stream, 1).await.remove(0);
            flagged = f;
            if f {
                after.push(r);
            } else {
                assert!(old_records.contains(&r));
            }
        }
        after.extend(read(&mut stream, 2).await.into_iter().map(|(r, f)| {
            assert!(!f);
            r
        }));
        assert_eq!(run_of(&after, &new_records), 0);
        assert!(s.va.metrics().reopens >= 1);
        let m = s.va.metrics().ingress;
        assert_eq!((m.sessions, m.replaced), (2, 1));
        assert_eq!(s.va.ingress_streams().len(), 1);
        let swarms = s.va.video_swarms();
        assert_eq!((swarms.len(), swarms[0].kind), (1, SwarmKind::Ingest));
        assert_eq!(s.a.host.swarms_of(&swarm_id(&s.channel)), (1, 0));
        assert!(!second.closed);
    });
}

/// A changed channel record serves the snippet again with the same start
/// time; a channel moved to another network, and a deleted one, end their
/// sessions.
#[test]
fn channel_changes_reach_the_session() {
    run(async {
        let s = Ingest::new(true, Config::default()).await;
        let b = s.b.as_ref().unwrap();
        let (other_net, _) = owned_network(&s.a, &[], 11).await;
        let mut events = s.va.subscribe();
        let mut p = s.publish("10.0.0.1:1").await;
        let script = Script::new(&ScriptConfig::default());
        p.send_tags(&script.tags[..script.tags.len() / 3], &script)
            .await;
        wait_for(10, "the snippet", || shown_snippet(b).is_some()).await;
        let first = shown_snippet(b).unwrap();
        assert_eq!(first.title, "first");

        s.va.update_channel(s.channel.id, &s.net.public, snippet("second"))
            .await
            .unwrap();
        wait_for(10, "the changed snippet", || {
            shown_snippet(b).is_some_and(|x| x.title == "second")
        })
        .await;
        let second = shown_snippet(b).unwrap();
        assert_eq!((second.start_time, second.live), (first.start_time, true));
        let server =
            s.a.directory
                .metrics(&s.net.public)
                .unwrap()
                .server
                .unwrap();
        assert_eq!(
            (server.snippets_shown, server.snippets_unverified),
            (2, 0),
            "{server:?}"
        );
        assert_eq!(s.va.ingress_streams().len(), 1);

        p.send_tags(&script.tags[script.tags.len() / 3..], &script)
            .await;
        s.va.update_channel(s.channel.id, &other_net.public, snippet("moved"))
            .await
            .unwrap();
        let reason = "the channel moved to another network";
        assert_eq!(ended(&mut events).await, (s.channel.id, reason.to_owned()));
        assert_eq!(p.refusal().await, refusal(FAILED, reason));

        let channel = s.va.get_channel(s.channel.id).await.unwrap();
        assert_eq!(channel.key, s.channel.key);
        let mut p = TestPublisher::connect(&s.video, "10.0.0.1:2", "live", &s.key, &mut s.a.rng());
        p.answered().await;
        assert!(p.publisher.publish_accepted());
        s.va.delete_channel(s.channel.id).await.unwrap();
        let reason = "the channel was deleted";
        assert_eq!(ended(&mut events).await, (s.channel.id, reason.to_owned()));
        assert_eq!(p.refusal().await, refusal(FAILED, reason));
        assert!(s.va.ingress_streams().is_empty());
        assert!(s.va.video_swarms().is_empty());
    });
}

/// A failed bind is reported and retried; a changed address binds anew
/// and leaves the session going; disabling drops the listener and ends
/// every connection.
#[test]
fn the_listener_follows_the_config() {
    run(async {
        let world = World::default();
        let a = node_in(&world, 1).await;
        let (net, _) = owned_network(&a, &[], 10).await;
        let video = FakeVideo::default();
        video.0.borrow_mut().fail_listens = 1;
        let va = start(&a, video.clone(), Config::default());
        let mut events = va.subscribe();
        settle().await;
        assert!(video.listens().is_empty());

        va.set_ingress_config(enabled(ADDR)).await.unwrap();
        assert_eq!(
            event(&mut events, "the failure", |e| matches!(
                e,
                VideoEvent::IngressListenFailed { .. }
            ))
            .await,
            VideoEvent::IngressListenFailed {
                addr: ADDR.into(),
                error: "closed".into()
            }
        );
        assert_eq!(video.listening(), None);
        sleep_secs(29).await;
        assert_eq!(video.listens(), [ADDR]);
        sleep_secs(2).await;
        assert_eq!(video.listens(), [ADDR, ADDR]);
        assert_eq!(video.listening().as_deref(), Some(ADDR));
        let m = va.metrics().ingress;
        assert_eq!((m.listens, m.listen_errors), (1, 1));

        let channel = va.create_channel(&net.public, snippet("x")).await.unwrap();
        let key = va.channel_url(channel.id).await.unwrap().stream_key;
        let mut p = TestPublisher::connect(&video, "10.0.0.1:1", "live", &key, &mut a.rng());
        p.answered().await;
        assert!(p.publisher.publish_accepted());
        let script = Script::new(&ScriptConfig::default());
        let half = script.tags.len() / 2;
        p.send_tags(&script.tags[..half], &script).await;

        let moved = "127.0.0.1:1936";
        va.set_ingress_config(enabled(moved)).await.unwrap();
        wait_for(10, "the new listener", || {
            video.listening().as_deref() == Some(moved)
        })
        .await;
        assert_eq!(video.listens(), [ADDR, ADDR, moved]);
        let segments = va.metrics().ingress.segments;
        p.send_tags(&script.tags[half..], &script).await;
        assert!(va.metrics().ingress.segments > segments);
        assert!(!p.closed);
        assert_eq!(va.ingress_streams().len(), 1);

        let mut idle = video.connect("10.0.0.2:1").unwrap();
        va.set_ingress_config(VideoIngressConfig {
            enabled: false,
            ..enabled(moved)
        })
        .await
        .unwrap();
        let reason = "the ingress was disabled";
        assert_eq!(ended(&mut events).await, (channel.id, reason.to_owned()));
        assert_eq!(p.refusal().await, refusal(FAILED, reason));
        wait_for(5, "the idle connection to close", || idle.closed()).await;
        assert_eq!(video.listening(), None);
        assert!(va.ingress_streams().is_empty());

        va.set_ingress_config(enabled(ADDR)).await.unwrap();
        wait_for(10, "the listener again", || {
            video.listening().as_deref() == Some(ADDR)
        })
        .await;
    });
}

/// A connection has 10 seconds to finish the handshake and 20 to
/// publish; a publishing one ends after 30 without media.
#[test]
fn deadlines_close_connections() {
    run(async {
        let s = Ingest::new(false, Config::default()).await;
        let mut events = s.va.subscribe();

        let mut silent = s.video.connect("10.0.0.1:1").unwrap();
        tokio::time::sleep(Duration::from_millis(9_900)).await;
        assert!(!silent.closed());
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(silent.closed());

        let mut p = TestPublisher::connect(&s.video, "10.0.0.2:1", "live", &s.key, &mut s.a.rng());
        p.until("the handshake", |p| p.publisher.handshake_done())
            .await;
        // The connect goes out, and its answer is never taken.
        p.flush().await;
        sleep_secs(15).await;
        assert!(!p.conn.closed());
        sleep_secs(6).await;
        assert!(p.conn.closed());

        let mut p = s.publish("10.0.0.3:1").await;
        let script = Script::new(&ScriptConfig::default());
        p.send_tags(&script.tags[..20], &script).await;
        sleep_secs(29).await;
        assert_eq!(s.va.ingress_streams().len(), 1);
        let reason = "no media for 30 s";
        assert_eq!(ended(&mut events).await, (s.channel.id, reason.to_owned()));
        assert_eq!(p.refusal().await, refusal(FAILED, reason));
        assert_eq!(s.va.metrics().ingress.deadlines, 3);
    });
}

/// Connections past the total cap, and unpublished ones past the cap per
/// remote host, close as they come.
#[test]
fn connections_are_capped() {
    run(async {
        let config = Config {
            ingress: IngressSettings {
                max_connections: 3,
                max_unpublished_per_address: 1,
                ..IngressSettings::default()
            },
            ..Config::default()
        };
        let s = Ingest::new(false, config).await;
        let settled = || tokio::time::sleep(Duration::from_millis(50));

        let mut p = TestPublisher::connect(&s.video, "10.0.0.1:1", "live", &s.key, &mut s.a.rng());
        settled().await;
        let mut same_host = s.video.connect("10.0.0.1:2").unwrap();
        let mut other_host = s.video.connect("10.0.0.2:1").unwrap();
        settled().await;
        assert!(same_host.closed());
        assert!(!other_host.closed());

        p.answered().await;
        assert!(p.publisher.publish_accepted());
        let mut after_publish = s.video.connect("10.0.0.1:3").unwrap();
        settled().await;
        assert!(!after_publish.closed());
        let mut over = s.video.connect("10.0.0.3:1").unwrap();
        settled().await;
        assert!(over.closed());
        let m = s.va.metrics().ingress;
        assert_eq!((m.connections, m.rejected_connections), (3, 2));
    });
}

/// `ListStreams` lists the live sessions, each updated as it writes
/// records.
#[test]
fn list_streams_reports_live_sessions() {
    run(async {
        let s = Ingest::new(false, Config::default()).await;
        let router = Router::new();
        bind_frontend(&router, &s.va);
        let ingress = VideoIngressClient::new(frontend_client(router));
        let list = || async {
            ingress
                .list_streams(&VideoIngressListStreamsRequest {})
                .await
                .unwrap()
                .streams
        };
        assert!(list().await.is_empty());

        let mut events = s.va.subscribe();
        let at = s.a.timer.now().unix();
        let mut p = s.publish("10.0.0.1:1").await;
        let streams = list().await;
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].channel_id, s.channel.id);
        assert!(streams[0].created_at >= at && streams[0].created_at <= at + 1);
        assert_eq!(streams[0].updated_at, streams[0].created_at);

        let script = Script::new(&ScriptConfig::default());
        let half = script.tags.len() / 2;
        sleep_secs(3).await;
        p.send_tags(&script.tags[..half], &script).await;
        let streams = list().await;
        assert!(streams[0].updated_at >= streams[0].created_at + 3);
        assert_eq!(streams[0].id, s.va.ingress_streams()[0].id);

        p.unpublish().await;
        ended(&mut events).await;
        assert!(list().await.is_empty());
    });
}

/// A node whose ingest writes fMP4 records.
fn fmp4_config() -> Config {
    Config {
        ingress: IngressSettings {
            format: IngestFormat::Fmp4,
            ..IngressSettings::default()
        },
        ..Config::default()
    }
}

fn units_config() -> Config {
    Config {
        ingress: IngressSettings {
            format: IngestFormat::Units,
            ..IngressSettings::default()
        },
        ..Config::default()
    }
}

/// The URI of a channel's swarm of the unit format, signing with
/// `signing`.
fn units_uri(c: &VideoChannel, signing: &UnitSigning) -> String {
    Uri::new(swarm_id(c), unit_ingress_options(signing).uri_options()).to_string()
}

/// A unit as the source made it, without its publish time: track, DTS,
/// PTS, keyframe and payload.
type Unit = (TrackId, i64, i64, bool, Vec<u8>);

fn owned(u: &AccessUnit<'_>) -> Unit {
    (u.track, u.dts, u.pts, u.keyframe, u.payload.to_vec())
}

/// The decoder configs and units the source makes of `script`, in order.
fn script_units(script: &Script) -> (Vec<DecoderConfig>, Vec<Unit>) {
    let mut source = FlvSource::new();
    let (mut configs, mut units) = (Vec::new(), Vec::new());
    let at = Time::from_unix_millis(0);
    for tag in &script.tags {
        let e = match tag.kind {
            TagKind::Video => source.video(tag.timestamp, &tag.body, at),
            TagKind::Audio => source.audio(tag.timestamp, &tag.body, at),
            TagKind::Metadata => continue,
        };
        match e.unwrap() {
            Some(SourceEvent::Config(c)) => configs.push(c),
            Some(SourceEvent::Unit(u)) => units.push(owned(&u)),
            None => {}
        }
    }
    (configs, units)
}

/// The default script's audio alone, without its metadata.
fn audio_only() -> Script {
    let mut script = Script::new(&ScriptConfig::default());
    script.tags.retain(|t| t.kind == TagKind::Audio);
    script
}

/// An `onMetaData` tag: a publisher sends a script's metadata for it.
fn metadata_tag() -> Tag {
    Tag {
        kind: TagKind::Metadata,
        timestamp: 0,
        body: Vec::new(),
    }
}

/// The script of a stream of 400 kbit/s video and 96 kbit/s audio.
fn low_bitrate(duration_secs: u32) -> Script {
    Script::new(&ScriptConfig {
        video_bitrate: 400_000,
        audio_bitrate: 96_000,
        duration_secs,
        ..ScriptConfig::default()
    })
}

/// A stream of the unit format read through a unit reader.
struct UnitViewer<V: VideoHost> {
    stream: Stream<V>,
    reader: UnitReader,
    record: Vec<u8>,
    flagged: bool,
    /// The units read, each with whether a discontinuity came before it.
    units: Vec<(Unit, bool)>,
    /// The units the reader's gaps counted.
    missing: u64,
}

impl<V: VideoHost> UnitViewer<V> {
    fn new(stream: Stream<V>) -> UnitViewer<V> {
        UnitViewer {
            stream,
            reader: UnitReader::new(),
            record: Vec::new(),
            flagged: false,
            units: Vec::new(),
            missing: 0,
        }
    }

    /// Reads until `done` holds of the units read.
    async fn until(&mut self, done: impl Fn(&[(Unit, bool)]) -> bool) {
        while !done(&self.units) {
            let piece = tokio::time::timeout(Duration::from_secs(10), self.stream.next(usize::MAX))
                .await
                .expect("a piece in time")
                .expect("a piece");
            match piece {
                EgressPiece::Data(d) => self.record.extend_from_slice(d),
                EgressPiece::RecordEnd => {
                    let r = std::mem::take(&mut self.record);
                    if let Some(u) = self.reader.read(&r) {
                        self.units
                            .push((owned(&u), std::mem::take(&mut self.flagged)));
                    }
                    while let Some(e) = self.reader.poll_event() {
                        if let ReaderEvent::Gap { missing, .. } = e {
                            self.missing += missing;
                        }
                    }
                }
                EgressPiece::Discontinuity => {
                    self.record.clear();
                    self.reader.lost();
                    self.flagged = true;
                }
            }
        }
    }

    /// Reads until the unit read last is `last`.
    async fn until_unit(&mut self, last: &Unit) {
        self.until(|u| u.last().is_some_and(|(u, _)| u == last))
            .await;
    }

    /// The units read.
    fn units(&self) -> Vec<Unit> {
        self.units.iter().map(|(u, _)| u.clone()).collect()
    }

    fn discontinuities(&self) -> usize {
        self.units.iter().filter(|(_, f)| *f).count()
    }
}

/// The media listing of network 10 on `n`, once it has the swarm URI
/// `uri`.
async fn listed_at(n: &Node, uri: &str) -> Media {
    wait_for(10, "the listing", || {
        media(n, 10)
            .first()
            .is_some_and(|(_, m, _)| m.swarm_uri == uri)
    })
    .await;
    media(n, 10).remove(0).1
}

/// A unit-format stream whose metadata states a bitrate of 750 kbit/s or
/// more has its swarm, of 8-chunk units, created on that metadata ahead
/// of any media, and a partial unit is signed 300 ms after its first
/// byte.
#[test]
fn a_stream_stating_a_high_bitrate_signs_eight_chunk_units() {
    run(async {
        let s = Ingest::new(true, units_config()).await;
        let b = s.b.as_ref().unwrap();
        let mut p = s.publish("10.0.0.1:1").await;
        assert!(s.va.video_swarms().is_empty());
        let script = Script::new(&ScriptConfig::default());
        assert_eq!(script.tags[0].kind, TagKind::Metadata);
        p.send_tags(&script.tags[..1], &script).await;
        let swarm = s.ingest_swarm().expect("the swarm, on the metadata");
        let large = units_uri(&s.channel, &UnitSigning::LARGE);
        assert_eq!(swarm.swarm_uri, large);
        assert_eq!(large, channel_uri(&s.channel));
        assert!(
            s.a.transfers
                .is_published(s.transfer_id(), s.a.net(&s.net.public))
        );
        assert_eq!(s.written(), 0);

        let third = script.tags.len() / 3;
        p.send_tags(&script.tags[1..third], &script).await;
        let before = s.va.metrics().ingress.early_signs;
        assert_ne!(s.written() % LARGE_UNIT, 0);
        tokio::time::sleep(Duration::from_millis(280)).await;
        assert_eq!(s.va.metrics().ingress.early_signs, before);
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(s.va.metrics().ingress.early_signs, before + 1);
        assert_eq!(s.written() % LARGE_UNIT, 0);

        let m = listed_at(b, &large).await;
        assert_eq!(m.mime_type, units_mime(&script_configs(&script)));
        p.send_tags(&script.tags[third..], &script).await;
        p.unpublish().await;
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        let m = s.va.metrics().ingress;
        assert_eq!((m.metadata, m.bytes), (1, s.written()));
        assert_eq!(s.written() % LARGE_UNIT, 0);
    });
}

/// A unit-format stream whose metadata states a bitrate below 750 kbit/s
/// signs 1-chunk units, a partial one 200 ms after its first byte, and
/// reaches another node's stream whole.
#[test]
fn a_stream_stating_a_low_bitrate_signs_single_chunk_units() {
    run(async {
        let s = Ingest::new(true, units_config()).await;
        let b = s.b.as_ref().unwrap();
        let vb = start(b, NoVideoHost, Config::default());
        let mut p = s.publish("10.0.0.1:1").await;
        let script = low_bitrate(6);
        let (configs, units) = script_units(&script);
        let third = script.tags.len() / 3;
        p.send_tags(&script.tags[..third], &script).await;
        let small = units_uri(&s.channel, &UnitSigning::SMALL);
        assert_eq!(s.ingest_swarm().unwrap().swarm_uri, small);

        let before = s.va.metrics().ingress.early_signs;
        assert_ne!(s.written() % CHUNK, 0);
        tokio::time::sleep(Duration::from_millis(180)).await;
        assert_eq!(s.va.metrics().ingress.early_signs, before);
        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(s.va.metrics().ingress.early_signs, before + 1);
        assert_eq!(s.written() % CHUNK, 0);

        let m = listed_at(b, &small).await;
        assert_eq!(m.mime_type, units_mime(&configs));
        let mut v = UnitViewer::new(
            vb.open_stream(
                &m.swarm_uri,
                std::slice::from_ref(&s.net.public),
                StreamFormat::from_mime(&m.mime_type),
            )
            .await
            .unwrap(),
        );
        p.send_tags(&script.tags[third..], &script).await;
        p.unpublish().await;
        v.until_unit(units.last().unwrap()).await;
        assert_eq!(v.units(), units);
        assert_eq!((v.discontinuities(), v.missing), (0, 0));
        assert_eq!(v.reader.configs(), configs);

        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        let m = s.va.metrics().ingress;
        assert_eq!(m.bytes, s.written());
        assert_eq!(s.written() % CHUNK, 0);
        assert_eq!((m.dropped_units, m.media_errors, m.write_errors), (0, 0, 0));
    });
}

/// A unit-format stream with video and without metadata signs 8-chunk
/// units, its swarm created on its video config ahead of any unit.
#[test]
fn a_stream_with_video_and_no_metadata_signs_eight_chunk_units() {
    run(async {
        let s = Ingest::new(false, units_config()).await;
        let mut p = s.publish("10.0.0.1:1").await;
        let mut script = Script::new(&ScriptConfig::default());
        script.tags.retain(|t| t.kind != TagKind::Metadata);
        // The video config.
        p.send_tags(&script.tags[..1], &script).await;
        let swarm = s.ingest_swarm().expect("the swarm, on the video config");
        assert_eq!(swarm.swarm_uri, units_uri(&s.channel, &UnitSigning::LARGE));
        assert_eq!(s.written(), 0);
        p.send_tags(&script.tags[1..], &script).await;
        p.unpublish().await;
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        let m = s.va.metrics().ingress;
        assert_eq!((m.metadata, m.bytes), (0, s.written()));
        assert_eq!(s.written() % LARGE_UNIT, 0);
        assert_eq!((m.dropped_units, m.media_errors, m.write_errors), (0, 0, 0));
    });
}

/// Audio alone without metadata waits from its first unit for
/// `signing_wait`, its units held meanwhile, then signs 1-chunk units, and
/// every unit reaches another node's stream.
#[test]
fn audio_alone_without_metadata_signs_single_chunk_units_at_the_deadline() {
    run(async {
        let s = Ingest::new(true, units_config()).await;
        let b = s.b.as_ref().unwrap();
        let vb = start(b, NoVideoHost, Config::default());
        let mut p = s.publish("10.0.0.1:1").await;
        let script = audio_only();
        let (configs, units) = script_units(&script);
        p.send_tags(&script.tags[..16], &script).await;
        let wait = IngressSettings::default().signing_wait.to_std().unwrap();
        tokio::time::sleep(wait - Duration::from_millis(100)).await;
        assert!(s.va.video_swarms().is_empty());
        assert_eq!((s.written(), s.va.metrics().ingress.bytes), (0, 0));
        tokio::time::sleep(Duration::from_millis(200)).await;
        let swarm = s.ingest_swarm().expect("the swarm, at the deadline");
        let small = units_uri(&s.channel, &UnitSigning::SMALL);
        assert_eq!(swarm.swarm_uri, small);
        assert!(s.written() > 0);

        let m = listed_at(b, &small).await;
        assert_eq!(m.mime_type, r#"video/x.strims-units;codecs="mp4a.40.2""#);
        assert_eq!(m.mime_type, units_mime(&configs));
        let mut v = UnitViewer::new(
            vb.open_stream(
                &m.swarm_uri,
                std::slice::from_ref(&s.net.public),
                StreamFormat::from_mime(&m.mime_type),
            )
            .await
            .unwrap(),
        );
        p.send_tags(&script.tags[16..], &script).await;
        p.unpublish().await;
        v.until_unit(units.last().unwrap()).await;
        assert_eq!(v.units(), units);
        assert_eq!((v.discontinuities(), v.missing), (0, 0));
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        let m = s.va.metrics().ingress;
        assert_eq!((m.metadata, m.bytes), (0, s.written()));
        assert_eq!((m.dropped_units, m.media_errors, m.write_errors), (0, 0, 0));
    });
}

/// Metadata that comes while audio alone waits settles the signing then:
/// here it states video and audio over 750 kbit/s, so 8-chunk units,
/// before the deadline.
#[test]
fn metadata_within_the_wait_settles_the_signing() {
    run(async {
        let s = Ingest::new(false, units_config()).await;
        let mut p = s.publish("10.0.0.1:1").await;
        let script = audio_only();
        p.send_tags(&script.tags[..16], &script).await;
        tokio::time::sleep(Duration::from_millis(200)).await;
        assert!(s.va.video_swarms().is_empty());
        let stated = Script::new(&ScriptConfig::default());
        p.send_tags(&[metadata_tag()], &stated).await;
        let swarm = s.ingest_swarm().expect("the swarm, on the metadata");
        assert_eq!(swarm.swarm_uri, units_uri(&s.channel, &UnitSigning::LARGE));
        assert!(s.written() > 0);
        p.send_tags(&script.tags[16..], &script).await;
        p.unpublish().await;
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        let m = s.va.metrics().ingress;
        assert_eq!((m.metadata, m.bytes), (1, s.written()));
        assert_eq!(s.written() % LARGE_UNIT, 0);
    });
}

/// Metadata after the wait changes nothing: the swarm, and the listing,
/// keep the signing the deadline settled.
#[test]
fn metadata_after_the_wait_changes_nothing() {
    run(async {
        let s = Ingest::new(true, units_config()).await;
        let b = s.b.as_ref().unwrap();
        let mut p = s.publish("10.0.0.1:1").await;
        let script = audio_only();
        p.send_tags(&script.tags[..16], &script).await;
        let wait = IngressSettings::default().signing_wait.to_std().unwrap();
        tokio::time::sleep(wait + Duration::from_millis(100)).await;
        let small = units_uri(&s.channel, &UnitSigning::SMALL);
        assert_eq!(s.ingest_swarm().unwrap().swarm_uri, small);

        let stated = Script::new(&ScriptConfig {
            video_bitrate: 6_000_000,
            ..ScriptConfig::default()
        });
        p.send_tags(&[metadata_tag()], &stated).await;
        p.send_tags(&script.tags[16..], &script).await;
        let swarms = s.va.video_swarms();
        assert_eq!(swarms.len(), 1);
        assert_eq!(swarms[0].swarm_uri, small);
        listed_at(b, &small).await;
        assert_eq!(s.va.metrics().ingress.metadata, 1);
        p.unpublish().await;
        wait_for(10, "the session's end", || {
            s.va.ingress_streams().is_empty()
        })
        .await;
        assert_eq!(s.va.metrics().ingress.bytes, s.written());
    });
}

/// A unit-format session replacing another signs as its own stream does:
/// the slot takes its swarm with its options, the listing its URI, and
/// the node's stream of the channel goes on after a discontinuity with
/// its units.
#[test]
fn a_session_replacing_another_signs_as_its_own_stream() {
    run(async {
        let s = Ingest::new(true, units_config()).await;
        let b = s.b.as_ref().unwrap();
        let mut events = s.va.subscribe();
        let mut first = s.publish("10.0.0.1:1").await;
        let old = Script::new(&ScriptConfig {
            duration_secs: 6,
            ..ScriptConfig::default()
        });
        let (_, old_units) = script_units(&old);
        first.send_tags(&old.tags[..old.tags.len() / 2], &old).await;
        let large = units_uri(&s.channel, &UnitSigning::LARGE);
        assert_eq!(s.ingest_swarm().unwrap().swarm_uri, large);
        listed_at(b, &large).await;
        let mut v = UnitViewer::new(
            s.va.open_stream(
                &large,
                std::slice::from_ref(&s.net.public),
                StreamFormat::Units,
            )
            .await
            .unwrap(),
        );
        v.until(|u| u.len() >= 10).await;
        assert_eq!(v.units(), old_units[..v.units.len()]);

        let mut second = s.publish("10.0.0.1:2").await;
        assert_eq!(first.refusal().await, refusal(FAILED, REPLACED));
        assert_eq!(
            ended(&mut events).await,
            (s.channel.id, REPLACED.to_owned())
        );
        let new = low_bitrate(4);
        let (_, new_units) = script_units(&new);
        second.send_tags(&new.tags, &new).await;
        let small = units_uri(&s.channel, &UnitSigning::SMALL);
        let swarms = s.va.video_swarms();
        assert_eq!(swarms.len(), 1);
        assert_eq!(
            (swarms[0].kind, &swarms[0].swarm_uri, swarms[0].streams),
            (SwarmKind::Ingest, &small, 1)
        );

        // Units of the first session may come before the discontinuity.
        v.until(|u| {
            u.iter()
                .position(|(_, f)| *f)
                .is_some_and(|i| u.len() >= i + 20)
        })
        .await;
        let at = v.units.iter().position(|(_, f)| *f).unwrap();
        let before = v.units();
        assert_eq!(before[..at], old_units[..at]);
        assert_eq!(before[at..], new_units[..before.len() - at]);
        assert_eq!(v.discontinuities(), 1);
        listed_at(b, &small).await;
        assert!(s.va.metrics().reopens >= 1);
        let m = s.va.metrics().ingress;
        assert_eq!((m.sessions, m.replaced), (2, 1));
        assert_eq!(s.a.host.swarms_of(&swarm_id(&s.channel)), (1, 0));
        assert!(!second.closed);
    });
}

/// A unit-format session that ends before its signing settles, never
/// listed, writes nothing and leaves no swarm, slot or transfer.
#[test]
fn a_session_ending_before_its_swarm_leaves_nothing() {
    run(async {
        let s = Ingest::new(false, units_config()).await;
        let mut events = s.va.subscribe();
        let mut p = s.publish("10.0.0.1:1").await;
        let script = audio_only();
        p.send_tags(&script.tags[..16], &script).await;
        tokio::time::sleep(Duration::from_millis(100)).await;
        p.unpublish().await;
        assert_eq!(
            ended(&mut events).await,
            (s.channel.id, "the publisher ended the stream".to_owned())
        );
        assert!(s.va.video_swarms().is_empty());
        assert_eq!(s.a.host.swarms_of(&swarm_id(&s.channel)), (0, 0));
        assert!(!s.a.transfers.list().contains(&s.transfer_id()));
        assert_eq!(s.written(), 0);
        let m = s.va.metrics().ingress;
        assert_eq!((m.sessions, m.segments, m.bytes), (1, 0, 0));
    });
}
