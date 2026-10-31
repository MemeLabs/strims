// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Autoseed on fake nodes (see `strims_testworld`), through the UI's RPC
//! connection: a node seeds the swarm of another node's capture as its
//! rules and config say, streams share the seeded swarm, salted rules
//! leave other components' transfers alone, and the frontend's records
//! and bounds.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::fmt::Debug;
use std::time::Duration;

use futures_util::StreamExt;
use strims_dao::Table;
use strims_dao::schema::network::Networks;
use strims_kv::MemoryKv;
use strims_ppspp::{SwarmId, Uri};
use strims_proto::autoseed::v1::{
    AutoseedFrontendClient, Config as SeedConfig, CreateRuleRequest, DeleteRuleRequest,
    GetConfigRequest, GetRuleRequest, ListRulesRequest, Rule, SetConfigRequest, UpdateRuleRequest,
};
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::{Listing, ListingSnippet};
use strims_proto::service::CallError;
use strims_proto::video::v1::capture_config::Codec;
use strims_proto::video::v1::egress_open_stream_response::Body;
use strims_proto::video::v1::{
    CaptureAppendRequest, CaptureClient, CaptureConfig, CaptureOpenRequest, CaptureUnit,
    EgressClient, EgressOpenStreamRequest, EgressOpenStreamResponse,
};
use strims_rpc::{Client, ClientStream, Router, Timer};
use strims_testworld::*;
use strims_transfer::{SwarmHost, TransferId};
use strims_video::hls::HlsRoutes;
use strims_video::synth::script::avcc;
use strims_video::units::UnitReader;
use strims_video::{
    AutoseedSettings, Config, NoVideoHost, SwarmKind, VideoControl, VideoSwarm, bind_frontend,
};

type Control = VideoControl<MemoryKv, FakeHost, NoVideoHost, TokioTimer>;

fn start(n: &Node, config: Config) -> Control {
    let video = VideoControl::new(
        n.host.clone(),
        NoVideoHost,
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
    let v = video.clone();
    n.spawn(async move { v.run().await });
    video
}

/// A UI's clients on one connection to `video`'s frontend.
struct Ui {
    autoseed: AutoseedFrontendClient<Client>,
    capture: CaptureClient<Client>,
    egress: EgressClient<Client>,
}

fn connect_ui(video: &Control) -> Ui {
    let router = Router::new();
    bind_frontend(&router, video);
    let client = frontend_client(router);
    Ui {
        autoseed: AutoseedFrontendClient::new(client.clone()),
        capture: CaptureClient::new(client.clone()),
        egress: EgressClient::new(client),
    }
}

/// The message of a call's remote error.
fn refusal<T: Debug>(r: Result<T, CallError>) -> String {
    match r {
        Err(CallError::Remote(e)) => e.message,
        r => panic!("not refused: {r:?}"),
    }
}

fn rule(network_key: &[u8], swarm: &SwarmId, salt: &[u8]) -> Rule {
    Rule {
        id: 0,
        network_key: network_key.to_vec(),
        swarm_id: swarm.as_bytes().to_vec(),
        salt: salt.to_vec(),
        label: "seed it".into(),
    }
}

impl Ui {
    async fn enable(&self, enable: bool) {
        let req = SetConfigRequest {
            config: Some(SeedConfig { enable }),
        };
        self.autoseed.set_config(&req).await.unwrap();
    }

    async fn create(&self, rule: Rule) -> Result<Rule, CallError> {
        let req = CreateRuleRequest { rule: Some(rule) };
        Ok(self.autoseed.create_rule(&req).await?.rule.unwrap())
    }

    async fn delete(&self, id: u64) {
        self.autoseed
            .delete_rule(&DeleteRuleRequest { id })
            .await
            .unwrap();
    }

    async fn rules(&self) -> Vec<Rule> {
        self.autoseed
            .list_rules(&ListRulesRequest {})
            .await
            .unwrap()
            .rules
    }

    /// Opens a capture listed on `key` and writes its first picture,
    /// [`first_picture`], so it is listed: its id and swarm.
    async fn open_capture(&self, video: &Control, key: &[u8], title: &str) -> (Vec<u8>, SwarmId) {
        let req = CaptureOpenRequest {
            directory_snippet: Some(ListingSnippet {
                title: title.into(),
                live: true,
                ..Default::default()
            }),
            network_keys: vec![key.to_vec()],
            bitrate: 2_000_000,
            video: true,
        };
        let id = self.capture.open(&req).await.unwrap().id;
        let config = CaptureConfig {
            track: 1,
            codec: Codec::H264 as i32,
            config: avcc(640, 360),
        };
        self.append_unit(&id, vec![config], 0, true, &first_picture())
            .await;
        let tid = TransferId::from_bytes(&id).unwrap();
        let c = video.captures().into_iter().find(|c| c.id == tid).unwrap();
        (id, Uri::parse(&c.swarm_uri).unwrap().id)
    }

    /// Appends a picture at `timestamp_us` to capture `id`.
    async fn append(&self, id: &[u8], timestamp_us: i64, data: &[u8]) {
        self.append_unit(id, Vec::new(), timestamp_us, false, data)
            .await;
    }

    async fn append_unit(
        &self,
        id: &[u8],
        configs: Vec<CaptureConfig>,
        timestamp_us: i64,
        keyframe: bool,
        data: &[u8],
    ) {
        let req = CaptureAppendRequest {
            id: id.to_vec(),
            configs,
            units: vec![CaptureUnit {
                track: 1,
                timestamp_us,
                keyframe,
                data: data.to_vec(),
            }],
        };
        self.capture.append(&req).await.unwrap();
    }
}

/// The media listings of network `network_id`'s view on `n`.
fn media(n: &Node, network_id: u64) -> Vec<Media> {
    n.directory
        .listings(network_id)
        .into_iter()
        .filter_map(|l| match l.listing.content {
            Some(Content::Media(m)) => Some(m),
            _ => None,
        })
        .collect()
}

/// The video swarm of `swarm` on `video`, if any.
fn video_swarm(video: &Control, swarm: &SwarmId) -> Option<VideoSwarm> {
    let t = TransferId::new(swarm, b"");
    video
        .video_swarms()
        .into_iter()
        .find(|s| s.transfer_id == t)
}

/// Whether `n` seeds `swarm` under the empty salt, published on `key`.
fn seeds(n: &Node, video: &Control, swarm: &SwarmId, key: &[u8]) -> bool {
    video_swarm(video, swarm).is_some_and(|s| {
        s.kind == SwarmKind::Seed
            && s.added
            && n.transfers
                .is_published(TransferId::new(swarm, b""), n.net(key))
    })
}

/// Two nodes on network 10, owned by `a`, each running its video, and
/// `a`'s UI connection.
struct Pair {
    a: Node,
    b: Node,
    key: Vec<u8>,
    va: Control,
    vb: Control,
    ua: Ui,
    ub: Ui,
    _pipes: Vec<Pipe>,
}

async fn pair() -> Pair {
    let world = World::default();
    let a = node_in(&world, 1).await;
    let b = node_in(&world, 2).await;
    let (net, pipes) = owned_network(&a, &[&b], 10).await;
    let va = start(&a, Config::default());
    let vb = start(&b, Config::default());
    let (ua, ub) = (connect_ui(&va), connect_ui(&vb));
    Pair {
        a,
        b,
        key: net.public,
        va,
        vb,
        ua,
        ub,
        _pipes: pipes,
    }
}

impl Pair {
    /// A capture on `a`, once `b` sees its listing: its id and swarm.
    async fn listed_capture(&self, title: &str) -> (Vec<u8>, SwarmId) {
        let listed = media(&self.b, 10).len();
        let c = self.ua.open_capture(&self.va, &self.key, title).await;
        wait_for(10, "the listing", || media(&self.b, 10).len() > listed).await;
        c
    }

    async fn wait_seeding(&self, swarm: &SwarmId) {
        wait_for(10, "the seed", || {
            seeds(&self.b, &self.vb, swarm, &self.key)
        })
        .await;
    }
}

/// A listing that is already in the view is seeded once autoseed is
/// enabled, and not before.
#[test]
fn enabling_autoseed_seeds_a_listing_that_already_exists() {
    run(async {
        let p = pair().await;
        let (_, swarm) = p.listed_capture("live").await;
        let r = p.ub.create(rule(&p.key, &swarm, b"")).await.unwrap();
        assert_ne!(r.id, 0);
        settle().await;
        assert!(p.vb.video_swarms().is_empty());

        p.ub.enable(true).await;
        p.wait_seeding(&swarm).await;
        let s = video_swarm(&p.vb, &swarm).unwrap();
        assert_eq!((s.seeds, s.streams), (1, 0));
        assert_eq!(s.network_keys, std::slice::from_ref(&p.key));
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 1));
    });
}

/// A media listing of `swarm`, of MIME type `mime`.
fn listing(swarm: &SwarmId, mime: &str) -> Listing {
    let uri = Uri::new(swarm.clone(), strims_video::capture_options().uri_options());
    Listing {
        content: Some(Content::Media(Media {
            mime_type: mime.into(),
            swarm_uri: uri.to_string(),
        })),
    }
}

/// The chunks per signature of `video`'s swarm of `swarm`.
fn chunks_per_signature(video: &Control, swarm: &SwarmId) -> usize {
    let s = video_swarm(video, swarm).unwrap();
    Uri::parse(&s.swarm_uri)
        .unwrap()
        .options
        .swarm_options()
        .chunks_per_signature
}

/// A seed takes its listing's options: a listing whose writer re-created
/// the swarm with others reopens the swarm a stream still holds with them.
#[test]
fn a_seed_reopens_the_swarm_with_its_listings_new_options() {
    run(async {
        let p = pair().await;
        let swarm = SwarmId::new(&[9; 32]);
        p.ub.enable(true).await;
        p.ub.create(rule(&p.key, &swarm, b"")).await.unwrap();
        let d = &p.a.directory;
        let first = listing(&swarm, strims_video::units::MIME_TYPE);
        let id = d.publish(&p.key, first.clone()).await.unwrap();
        p.wait_seeding(&swarm).await;
        assert_eq!(chunks_per_signature(&p.vb, &swarm), 8);
        let Some(Content::Media(m)) = &first.content else {
            unreachable!();
        };
        let mut s =
            p.ub.egress
                .open_stream(&EgressOpenStreamRequest {
                    swarm_uri: m.swarm_uri.clone(),
                    network_keys: vec![p.key.clone()],
                    mime_type: m.mime_type.clone(),
                })
                .await
                .unwrap();
        let Body::Open(_) = next(&mut s).await else {
            panic!("not open first");
        };

        d.unpublish(&p.key, id).await.unwrap();
        wait_for(10, "the seed to end", || {
            video_swarm(&p.vb, &swarm).is_some_and(|v| v.kind == SwarmKind::Egress)
        })
        .await;
        let options = strims_ppspp::SwarmOptions {
            chunks_per_signature: 1,
            ..strims_video::capture_options()
        };
        let uri = Uri::new(swarm.clone(), options.uri_options()).to_string();
        let second = Listing {
            content: Some(Content::Media(Media {
                mime_type: strims_video::units::MIME_TYPE.into(),
                swarm_uri: uri,
            })),
        };
        d.publish(&p.key, second).await.unwrap();
        p.wait_seeding(&swarm).await;
        assert_eq!(chunks_per_signature(&p.vb, &swarm), 1);
        let v = video_swarm(&p.vb, &swarm).unwrap();
        assert_eq!((v.seeds, v.streams), (1, 1));
        settle().await;
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 1));
    });
}

/// A rule seeds its swarm's listing as it appears, and stops as it goes.
#[test]
fn a_seed_follows_the_listing_of_its_swarm() {
    run(async {
        let p = pair().await;
        let swarm = SwarmId::new(&[9; 32]);
        let other = SwarmId::new(&[8; 32]);
        p.ub.enable(true).await;
        p.ub.create(rule(&p.key, &swarm, b"")).await.unwrap();
        settle().await;
        assert!(p.vb.video_swarms().is_empty());

        let d = &p.a.directory;
        d.publish(&p.key, listing(&other, "video/mp4"))
            .await
            .unwrap();
        let id = d
            .publish(&p.key, listing(&swarm, "video/mp4"))
            .await
            .unwrap();
        p.wait_seeding(&swarm).await;
        assert!(video_swarm(&p.vb, &other).is_none());
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 1));

        d.unpublish(&p.key, id).await.unwrap();
        wait_for(10, "the seed to end", || p.vb.video_swarms().is_empty()).await;
        settle().await;
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 0));

        d.publish(&p.key, listing(&swarm, strims_video::units::MIME_TYPE))
            .await
            .unwrap();
        p.wait_seeding(&swarm).await;
    });
}

/// A rule's swarm goes with the rule.
#[test]
fn deleting_a_rule_removes_its_swarm() {
    run(async {
        let p = pair().await;
        p.ub.enable(true).await;
        let (_, swarm) = p.listed_capture("live").await;
        let r = p.ub.create(rule(&p.key, &swarm, b"")).await.unwrap();
        p.wait_seeding(&swarm).await;

        p.ub.delete(r.id).await;
        wait_for(10, "the seed to end", || p.vb.video_swarms().is_empty()).await;
        assert!(!p.b.transfers.list().contains(&TransferId::new(&swarm, b"")));
        settle().await;
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 0));
    });
}

/// Disabling autoseed ends every seed, and enabling it again starts them
/// over.
#[test]
fn disabling_autoseed_removes_every_seed() {
    run(async {
        let p = pair().await;
        let (_, first) = p.listed_capture("first").await;
        let (_, second) = p.listed_capture("second").await;
        p.ub.create(rule(&p.key, &first, b"")).await.unwrap();
        p.ub.create(rule(&p.key, &second, b"")).await.unwrap();
        p.ub.enable(true).await;
        p.wait_seeding(&first).await;
        p.wait_seeding(&second).await;

        p.ub.enable(false).await;
        wait_for(10, "the seeds to end", || p.vb.video_swarms().is_empty()).await;
        settle().await;
        assert_eq!(p.b.host.swarms_of(&first), (0, 0));
        assert_eq!(p.b.host.swarms_of(&second), (0, 0));

        p.ub.enable(true).await;
        p.wait_seeding(&first).await;
        p.wait_seeding(&second).await;
    });
}

type Responses = ClientStream<EgressOpenStreamResponse>;

async fn next(s: &mut Responses) -> Body {
    match tokio::time::timeout(Duration::from_secs(10), s.next()).await {
        Ok(Some(Ok(r))) => r.body.unwrap(),
        r => panic!("no response: {r:?}"),
    }
}

/// The next record of a stream.
async fn read_record(s: &mut Responses) -> Vec<u8> {
    let mut rec = Vec::new();
    loop {
        let Body::Data(d) = next(s).await else {
            panic!("not data");
        };
        if d.discontinuity {
            rec.clear();
        }
        rec.extend_from_slice(&d.data);
        if d.segment_end {
            return rec;
        }
    }
}

fn record(i: u8, n: usize) -> Vec<u8> {
    (0..n)
        .map(|j| i.wrapping_mul(13).wrapping_add(j as u8))
        .collect()
}

/// The keyframe a capture of [`Ui::open_capture`] starts with.
fn first_picture() -> Vec<u8> {
    record(0, 2_000)
}

/// The payload of the next unit a unit-format stream's records carry.
async fn read_unit(s: &mut Responses, reader: &mut UnitReader) -> Vec<u8> {
    loop {
        let rec = read_record(s).await;
        if let Some(u) = reader.read(&rec) {
            return u.payload.to_vec();
        }
    }
}

/// A stream reads a seeded swarm where it is, and goes on reading it
/// once the seed ends; the swarm goes with the stream.
#[test]
fn a_stream_on_a_seeded_swarm_survives_deleting_the_rule() {
    run(async {
        let p = pair().await;
        p.ub.enable(true).await;
        let (id, swarm) = p.listed_capture("live").await;
        let r = p.ub.create(rule(&p.key, &swarm, b"")).await.unwrap();
        p.wait_seeding(&swarm).await;

        let listed = media(&p.b, 10).remove(0);
        let mut s =
            p.ub.egress
                .open_stream(&EgressOpenStreamRequest {
                    swarm_uri: listed.swarm_uri,
                    network_keys: vec![p.key.clone()],
                    mime_type: listed.mime_type,
                })
                .await
                .unwrap();
        let Body::Open(_) = next(&mut s).await else {
            panic!("not open first");
        };
        let v = video_swarm(&p.vb, &swarm).unwrap();
        assert_eq!((v.kind, v.seeds, v.streams), (SwarmKind::Seed, 1, 1));
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 1));
        let mut reader = UnitReader::new();
        assert_eq!(read_unit(&mut s, &mut reader).await, first_picture());
        let first = record(1, 70_000);
        p.ua.append(&id, 33_333, &first).await;
        assert_eq!(read_unit(&mut s, &mut reader).await, first);

        p.ub.delete(r.id).await;
        wait_for(10, "the seed to end", || {
            video_swarm(&p.vb, &swarm).is_some_and(|v| v.kind == SwarmKind::Egress)
        })
        .await;
        let v = video_swarm(&p.vb, &swarm).unwrap();
        assert_eq!((v.seeds, v.streams), (0, 1));
        let second = record(2, 80_000);
        p.ua.append(&id, 66_667, &second).await;
        assert_eq!(read_unit(&mut s, &mut reader).await, second);

        drop(s);
        wait_for(10, "the stream's swarm to go", || {
            p.vb.video_swarms().is_empty()
        })
        .await;
    });
}

/// A seed's end withdraws the swarm from its rule's network when no
/// stream of it names that network, while the stream holds the swarm.
#[test]
fn a_seed_that_ends_withdraws_its_network_from_a_swarm_a_stream_holds() {
    run(async {
        let p = pair().await;
        p.ub.enable(true).await;
        let (_, swarm) = p.listed_capture("live").await;
        let r = p.ub.create(rule(&p.key, &swarm, b"")).await.unwrap();
        p.wait_seeding(&swarm).await;

        let listed = media(&p.b, 10).remove(0);
        let mut s =
            p.ub.egress
                .open_stream(&EgressOpenStreamRequest {
                    swarm_uri: listed.swarm_uri,
                    network_keys: Vec::new(),
                    mime_type: listed.mime_type,
                })
                .await
                .unwrap();
        let Body::Open(_) = next(&mut s).await else {
            panic!("not open first");
        };
        let t = TransferId::new(&swarm, b"");
        assert!(p.b.transfers.is_published(t, p.b.net(&p.key)));

        p.ub.delete(r.id).await;
        wait_for(10, "the seed to end", || {
            video_swarm(&p.vb, &swarm).is_some_and(|v| v.kind == SwarmKind::Egress)
        })
        .await;
        assert!(!p.b.transfers.is_published(t, p.b.net(&p.key)));
        let v = video_swarm(&p.vb, &swarm).unwrap();
        assert_eq!((v.streams, v.network_keys.len()), (1, 0));
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 1));
    });
}

/// A rule under a non-empty salt seeds a transfer of its own under that
/// salt, and never another component's.
#[test]
fn a_non_empty_salt_leaves_another_components_transfer_alone() {
    run(async {
        let p = pair().await;
        let (_, swarm) = p.listed_capture("live").await;
        let other =
            p.b.host
                .add_swarm(swarm.clone(), strims_video::capture_options())
                .await
                .unwrap();
        let theirs = p.b.transfers.add(other, &swarm, b"theirs".to_vec());
        let taken = p.ub.create(rule(&p.key, &swarm, b"theirs")).await.unwrap();
        let own = p.ub.create(rule(&p.key, &swarm, b"own")).await.unwrap();
        p.ub.enable(true).await;

        let mine = TransferId::new(&swarm, b"own");
        wait_for(10, "the salted seed", || {
            p.b.transfers.is_published(mine, p.b.net(&p.key))
        })
        .await;
        assert_eq!(p.vb.metrics().autoseed_skipped, 1);
        assert_eq!(p.b.transfers.find(&swarm, b"theirs"), Some((theirs, other)));
        assert!(!p.b.transfers.is_published(theirs, p.b.net(&p.key)));
        assert!(p.vb.video_swarms().is_empty());
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 2));

        p.ub.delete(own.id).await;
        p.ub.delete(taken.id).await;
        wait_for(10, "the salted seed to end", || {
            p.b.transfers.find(&swarm, b"own").is_none()
        })
        .await;
        settle().await;
        assert_eq!(p.b.transfers.find(&swarm, b"theirs"), Some((theirs, other)));
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 1));
    });
}

/// Deleting a network deletes its rules, and their seeds end.
#[test]
fn deleting_the_network_stops_its_seeds() {
    run(async {
        let p = pair().await;
        p.ub.enable(true).await;
        let (_, swarm) = p.listed_capture("live").await;
        p.ub.create(rule(&p.key, &swarm, b"")).await.unwrap();
        p.wait_seeding(&swarm).await;

        let mut tx = p.b.store.write(p.b.timer.now()).await.unwrap();
        Networks::delete(&mut tx, 10).await.unwrap();
        tx.commit().await.unwrap();
        wait_for(10, "the seed to end", || p.vb.video_swarms().is_empty()).await;
        assert!(p.ub.rules().await.is_empty());
        settle().await;
        assert_eq!(p.b.host.swarms_of(&swarm), (0, 0));
    });
}

/// A rule naming a capture's swarm seeds it where it is: the capture's
/// swarm goes on the rule's network too, and leaves it as the rule goes,
/// staying on the capture's own.
#[test]
fn a_rule_naming_a_captures_swarm_publishes_it_on_the_rules_network() {
    run(async {
        let p = pair().await;
        let (n11, _) = owned_network(&p.a, &[], 11).await;
        p.ua.enable(true).await;
        let (_, swarm) = p.listed_capture("live").await;
        let uri = video_swarm(&p.va, &swarm).unwrap().swarm_uri;
        p.a.directory
            .publish(
                &n11.public,
                Listing {
                    content: Some(Content::Media(Media {
                        mime_type: strims_video::units::MIME_TYPE.into(),
                        swarm_uri: uri,
                    })),
                },
            )
            .await
            .unwrap();
        wait_for(10, "the listing on 11", || !media(&p.a, 11).is_empty()).await;
        let t = TransferId::new(&swarm, b"");
        assert!(!p.a.transfers.is_published(t, p.a.net(&n11.public)));
        let r = p.ua.create(rule(&n11.public, &swarm, b"")).await.unwrap();
        wait_for(10, "the seed", || {
            p.a.transfers.is_published(t, p.a.net(&n11.public))
        })
        .await;
        let v = video_swarm(&p.va, &swarm).unwrap();
        assert_eq!((v.kind, v.seeds), (SwarmKind::Capture, 1));
        assert_eq!(p.va.metrics().autoseed_skipped, 0);

        p.ua.delete(r.id).await;
        wait_for(10, "the seed to end", || {
            video_swarm(&p.va, &swarm).is_some_and(|v| v.seeds == 0)
        })
        .await;
        assert!(!p.a.transfers.is_published(t, p.a.net(&n11.public)));
        assert!(p.a.transfers.is_published(t, p.a.net(&p.key)));
        assert_eq!(video_swarm(&p.va, &swarm).unwrap().kind, SwarmKind::Capture);
    });
}

/// The frontend stores the config and rules, gives each rule a new id,
/// and refuses rules out of their bounds.
#[test]
fn the_frontend_keeps_the_config_and_rules() {
    run(async {
        let world = World::default();
        let n = node_in(&world, 1).await;
        let config = Config {
            autoseed: AutoseedSettings {
                max_rules: 2,
                ..AutoseedSettings::default()
            },
            ..Config::default()
        };
        let v = start(&n, config);
        let ui = connect_ui(&v);
        let a = &ui.autoseed;

        let c = a.get_config(&GetConfigRequest {}).await.unwrap();
        assert_eq!(c.config, Some(SeedConfig { enable: false }));
        ui.enable(true).await;
        let c = a.get_config(&GetConfigRequest {}).await.unwrap();
        assert_eq!(c.config, Some(SeedConfig { enable: true }));

        let swarm = SwarmId::new(&[7; 32]);
        let r = ui
            .create(Rule {
                id: 999,
                ..rule(b"key", &swarm, b"salt")
            })
            .await
            .unwrap();
        assert_ne!(r.id, 999);
        assert_eq!(
            r,
            Rule {
                id: r.id,
                ..rule(b"key", &swarm, b"salt")
            }
        );
        assert_eq!(ui.rules().await, std::slice::from_ref(&r));
        let got = a.get_rule(&GetRuleRequest { id: r.id }).await.unwrap();
        assert_eq!(got.rule, Some(r.clone()));

        let changed = Rule {
            id: 12345,
            label: "renamed".into(),
            ..rule(b"other", &swarm, b"")
        };
        let req = UpdateRuleRequest {
            id: r.id,
            rule: Some(changed.clone()),
        };
        let updated = a.update_rule(&req).await.unwrap().rule.unwrap();
        assert_eq!(
            updated,
            Rule {
                id: r.id,
                ..changed.clone()
            }
        );
        assert_eq!(ui.rules().await, std::slice::from_ref(&updated));

        let missing = r.id + 1000;
        assert_eq!(
            refusal(a.get_rule(&GetRuleRequest { id: missing }).await),
            "autoseed rule not found"
        );
        let req = UpdateRuleRequest {
            id: missing,
            rule: Some(changed.clone()),
        };
        assert_eq!(
            refusal(a.update_rule(&req).await),
            "autoseed rule not found"
        );
        a.delete_rule(&DeleteRuleRequest { id: missing })
            .await
            .unwrap();

        for (bad, message) in [
            (rule(b"", &swarm, b""), "invalid autoseed rule: network key"),
            (
                rule(&[1; 65], &swarm, b""),
                "invalid autoseed rule: network key",
            ),
            (
                rule(b"key", &SwarmId::new(&[]), b""),
                "invalid autoseed rule: swarm id",
            ),
            (
                rule(b"key", &SwarmId::new(&[1; 65]), b""),
                "invalid autoseed rule: swarm id",
            ),
            (
                rule(b"key", &swarm, &[1; 65]),
                "invalid autoseed rule: salt",
            ),
            (
                Rule {
                    label: "l".repeat(257),
                    ..rule(b"key", &swarm, b"")
                },
                "invalid autoseed rule: label",
            ),
        ] {
            assert_eq!(refusal(ui.create(bad.clone()).await), message);
            let req = UpdateRuleRequest {
                id: r.id,
                rule: Some(bad),
            };
            assert_eq!(refusal(a.update_rule(&req).await), message);
        }
        let empty = a.create_rule(&CreateRuleRequest { rule: None }).await;
        assert_eq!(refusal(empty), "invalid autoseed rule: network key");

        ui.create(rule(b"key", &swarm, b"second")).await.unwrap();
        assert_eq!(
            refusal(ui.create(rule(b"key", &swarm, b"third")).await),
            "too many autoseed rules"
        );
        a.delete_rule(&DeleteRuleRequest { id: r.id })
            .await
            .unwrap();
        assert_eq!(ui.rules().await.len(), 1);
        ui.create(rule(b"key", &swarm, b"third")).await.unwrap();
    });
}
