// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Players' feeds in a dedicated worker: a capture on the worker's own
//! node, read back through the feeds port as a player worker reads it,
//! against `Manager`, `WasmFactory` and `IdbKv` as `entry::start` builds
//! them. The test drives the profile over a frontend connection, as the
//! UI does, and plays the window and the player at the ports' other ends.
//!
//! Each test's databases share a prefix of their own, and the test
//! deletes them at its end.

#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::{Cell, RefCell};
use std::future::Future;
use std::rc::Rc;

use js_sys::{Array, ArrayBuffer, Function, Object, Promise, Reflect, Uint8Array};
use rand_core::SeedableRng;
use strims_base::rng::DetRng;
use strims_base::time::Duration;
use strims_crypto::CertificateChain;
use strims_kv::Kv;
use strims_proto::auth::v1::{AuthFrontendClient, SignUpRequest};
use strims_proto::network::v1::directory::listing::Content;
use strims_proto::network::v1::directory::{
    DirectoryFrontendClient, FrontendGetListingsRequest, ListingContentType, ListingSnippet,
};
use strims_proto::network::v1::{CreateServerRequest, NetworkFrontendClient};
use strims_proto::video::v1::capture_config::Codec;
use strims_proto::video::v1::{
    CaptureAppendRequest, CaptureClient, CaptureCloseRequest, CaptureConfig, CaptureOpenRequest,
    CaptureUnit,
};
use strims_rpc::{Client, Config as RpcConfig, Timer, connect};
use strims_video::synth::script::avcc;
use strims_video::units::{ReaderEvent, UnitReader};
use strims_wasm::IdbKv;
use strims_wasm::clock::{self, WasmClock};
use strims_wasm::entry::{DEVICE_OS, WasmManager};
use strims_wasm::factory::WasmFactory;
use strims_wasm::feed::{
    BATCH_ACK, BATCH_DATA, BATCH_END, DATA_DISCONTINUITY, DATA_RECORD_END, MAX_BATCH_BYTES,
    WINDOW_BATCHES, WINDOW_BYTES,
};
use strims_wasm::options::Options;
use strims_wasm::runtime::{self, TaskHandle};
use strims_wasm::{entropy, frontend};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::{MessageChannel, MessageEvent, MessagePort};

wasm_bindgen_test_configure!(run_in_dedicated_worker);

const OPUS_HEAD: [u8; 19] = *b"OpusHead\x01\x02\x38\x01\x80\xbb\0\0\0\0\0";
const VIDEO: u32 = 1;
const AUDIO: u32 = 2;
/// How long any one step may take.
const STEP: Duration = Duration::from_secs(30);

// --- the profile ----------------------------------------------------------------

/// A database-name prefix no other test or run shares.
fn unique(test: &str) -> String {
    let seed = entropy::seed().unwrap();
    let tag: String = seed[..6].iter().map(|b| format!("{b:02x}")).collect();
    format!("strims-wasm-test/feed/{test}/{tag}/")
}

async fn database_names() -> Vec<String> {
    let idb = Reflect::get(&js_sys::global(), &JsValue::from_str("indexedDB")).unwrap();
    let databases: Function = Reflect::get(&idb, &JsValue::from_str("databases"))
        .unwrap()
        .dyn_into()
        .unwrap();
    let list: Promise = databases.call0(&idb).unwrap().dyn_into().unwrap();
    let list = JsFuture::from(list).await.unwrap();
    Array::from(&list)
        .iter()
        .filter_map(|d| {
            Reflect::get(&d, &JsValue::from_str("name"))
                .ok()?
                .as_string()
        })
        .collect()
}

async fn sleep(d: Duration) {
    WasmClock.sleep_until(clock::now() + d).await;
}

/// `f`'s answer once it has one, polled every 100 ms, or a panic naming
/// `what` after [`STEP`].
async fn eventually<T, F, Fut>(what: &str, mut f: F) -> T
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Option<T>>,
{
    let deadline = clock::now() + STEP;
    loop {
        if let Some(v) = f().await {
            return v;
        }
        assert!(clock::now() < deadline, "timed out waiting for {what}");
        sleep(Duration::from_millis(100)).await;
    }
}

/// A signed-in profile with a network, served over a frontend connection,
/// whose factory serves feeds on `feeds`' other end.
struct Profile {
    prefix: String,
    manager: Rc<WasmManager>,
    client: Client,
    network_key: Vec<u8>,
    tasks: Vec<TaskHandle>,
}

impl Profile {
    async fn start(test: &str, feeds: MessagePort) -> Profile {
        let prefix = unique(test);
        let options = Options {
            ice_servers: Some(Vec::new()),
            ..Options::default()
        };
        let manager = Rc::new(WasmManager::new(
            IdbKv::new(&prefix),
            WasmFactory::new(
                &options,
                MessageChannel::new().unwrap().port1(),
                None,
                Some(feeds),
            ),
            WasmClock,
            DEVICE_OS,
            DetRng::from_seed(entropy::seed().unwrap()),
        ));
        let channel = MessageChannel::new().unwrap();
        let serve = runtime::spawn({
            let manager = manager.clone();
            let port = channel.port1();
            async move {
                let _ = frontend::serve(manager, port).await;
            }
        });
        let (reader, writer) = frontend::transport(channel.port2());
        let (client, conn) = connect(RpcConfig::frontend(), reader, writer, None);
        let conn = runtime::spawn(async move {
            let _ = conn.await;
        });
        AuthFrontendClient::new(client.clone())
            .sign_up(&SignUpRequest {
                name: "alice".to_owned(),
                password: "pw".to_owned(),
                persist_session: false,
                persist_login: false,
            })
            .await
            .unwrap();
        let network = NetworkFrontendClient::new(client.clone())
            .create_server(&CreateServerRequest {
                name: "net".into(),
                ..Default::default()
            })
            .await
            .unwrap()
            .network
            .unwrap();
        let network_key = CertificateChain::decode(network.certificate.as_ref().unwrap())
            .unwrap()
            .network_key()
            .to_vec();
        Profile {
            prefix,
            manager,
            client,
            network_key,
            tasks: vec![serve, conn],
        }
    }

    fn capture(&self) -> CaptureClient<Client> {
        CaptureClient::new(self.client.clone())
    }

    /// Opens a capture and writes its configs and first keyframes: its id.
    async fn open_capture(&self) -> Vec<u8> {
        let id = self
            .capture()
            .open(&CaptureOpenRequest {
                directory_snippet: Some(ListingSnippet {
                    title: "feed".into(),
                    description: "desc".into(),
                    channel_name: "chan".into(),
                    live: true,
                    ..Default::default()
                }),
                network_keys: vec![self.network_key.clone()],
                bitrate: 2_628_000,
                video: true,
            })
            .await
            .unwrap()
            .id;
        let configs = vec![
            CaptureConfig {
                track: VIDEO,
                codec: Codec::H264 as i32,
                config: avcc(640, 360),
            },
            CaptureConfig {
                track: AUDIO,
                codec: Codec::Opus as i32,
                config: OPUS_HEAD.to_vec(),
            },
        ];
        self.append(&id, configs, first_units()).await;
        id
    }

    async fn append(&self, id: &[u8], configs: Vec<CaptureConfig>, units: Vec<CaptureUnit>) {
        self.capture()
            .append(&CaptureAppendRequest {
                id: id.to_vec(),
                configs,
                units,
            })
            .await
            .unwrap();
    }

    async fn close_capture(&self, id: &[u8]) {
        self.capture()
            .close(&CaptureCloseRequest { id: id.to_vec() })
            .await
            .unwrap();
    }

    /// The capture's swarm URI and MIME type, from its listing.
    async fn listing(&self) -> (String, String) {
        let directory = DirectoryFrontendClient::new(self.client.clone());
        let key = self.network_key.clone();
        eventually("the capture's listing", || {
            let directory = directory.clone();
            let key = key.clone();
            async move {
                let res = directory
                    .get_listings(&FrontendGetListingsRequest {
                        content_types: vec![ListingContentType::Media as i32],
                        network_keys: vec![key],
                    })
                    .await
                    .ok()?;
                res.listings
                    .iter()
                    .flat_map(|n| &n.listings)
                    .find_map(|item| match item.listing.as_ref()?.content.as_ref()? {
                        Content::Media(m) => Some((m.swarm_uri.clone(), m.mime_type.clone())),
                        _ => None,
                    })
            }
        })
        .await
    }

    /// Stops the profile and deletes its databases.
    async fn shut_down(self) {
        for t in &self.tasks {
            t.abort();
        }
        let Profile {
            prefix, manager, ..
        } = self;
        drop(manager);
        sleep(Duration::from_millis(100)).await;
        let kv = IdbKv::new("");
        for name in database_names().await {
            if name.starts_with(&prefix) {
                kv.delete_table(&name).await.unwrap();
            }
        }
    }
}

/// A unit of `n` bytes, its first naming its track and time.
fn unit(track: u32, timestamp_us: i64, keyframe: bool, n: usize) -> CaptureUnit {
    let mut data: Vec<u8> = (0..n)
        .map(|j| (timestamp_us as usize / 1000).wrapping_add(j) as u8)
        .collect();
    data[0] = track as u8;
    CaptureUnit {
        track,
        timestamp_us,
        keyframe,
        data,
    }
}

fn first_units() -> Vec<CaptureUnit> {
    vec![unit(VIDEO, 0, true, 2_000), unit(AUDIO, 0, true, 160)]
}

/// `secs` of media after the first units: 30 fps video with no keyframe,
/// and 20 ms Opus packets, in time order, the video's `video_bytes` each.
fn media_units(secs: i64, video_bytes: usize) -> Vec<CaptureUnit> {
    let mut units = Vec::new();
    let end = secs * 1_000_000;
    let (mut frame, mut v, mut a) = (1, 1_000_000 / 30, 20_000);
    while v < end || a < end {
        if v <= a && v < end {
            units.push(unit(VIDEO, v, false, video_bytes));
            frame += 1;
            v = frame * 1_000_000 / 30;
        } else {
            units.push(unit(AUDIO, a, true, 160));
            a += 20_000;
        }
    }
    units
}

// --- the window and the player ---------------------------------------------------

/// The window's end of the feeds port: opens and closes feeds, and hears
/// which closed.
struct Window {
    port: MessagePort,
    closed: Rc<RefCell<Vec<(u64, String)>>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl Window {
    fn new(port: MessagePort) -> Window {
        let closed = Rc::new(RefCell::new(Vec::new()));
        let c = closed.clone();
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let d = e.data();
            let get = |k: &str| Reflect::get(&d, &JsValue::from_str(k)).unwrap();
            assert_eq!(get("type").as_string().as_deref(), Some("closed"));
            c.borrow_mut().push((
                get("id").as_f64().unwrap() as u64,
                get("message").as_string().unwrap(),
            ));
        });
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        Window {
            port,
            closed,
            _on_message: on_message,
        }
    }

    /// Opens feed `id` of `listing` for a new player.
    fn open(
        &self,
        id: u64,
        listing: &(String, String),
        key: &[u8],
        replaces: Option<u64>,
    ) -> Player {
        let channel = MessageChannel::new().unwrap();
        let o = Object::new();
        let set = |k: &str, v: &JsValue| {
            Reflect::set(&o, &JsValue::from_str(k), v).unwrap();
        };
        set("type", &"open".into());
        set("id", &JsValue::from_f64(id as f64));
        set("swarmUri", &listing.0.as_str().into());
        set("mimeType", &listing.1.as_str().into());
        set("networkKeys", &Array::of1(&Uint8Array::from(key)));
        set("port", &channel.port1());
        if let Some(r) = replaces {
            set("replaces", &JsValue::from_f64(r as f64));
        }
        self.port
            .post_message_with_transferable(&o, &Array::of1(&channel.port1()))
            .unwrap();
        Player::new(channel.port2())
    }

    fn close(&self, id: u64) {
        let o = Object::new();
        Reflect::set(&o, &"type".into(), &"close".into()).unwrap();
        Reflect::set(&o, &"id".into(), &JsValue::from_f64(id as f64)).unwrap();
        self.port.post_message(&o).unwrap();
    }
}

/// What a player worker does with a feed: takes its batches, acks those
/// that ask while `acking`, and reads the records back to units.
struct Player {
    port: MessagePort,
    state: Rc<RefCell<Taken>>,
    acking: Rc<Cell<bool>>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

#[derive(Default)]
struct Taken {
    batches: u64,
    /// Batches that asked for an ack, the last one's count.
    asked: u64,
    bytes: usize,
    ended: bool,
    discontinuities: u64,
    /// END records read.
    ends: u64,
    record: Vec<u8>,
    reader: UnitReader,
    /// The reader's events and units, in order: `None` for a configs
    /// event, the unit's track, keyframe and payload otherwise.
    read: Vec<Option<(u32, bool, Vec<u8>)>>,
}

impl Taken {
    fn take(&mut self, batch: &[u8]) {
        assert!(!self.ended, "a batch after the end");
        self.batches += 1;
        self.bytes += batch.len();
        match batch[0] {
            BATCH_END => {
                assert_eq!(batch.len(), 1);
                self.ended = true;
            }
            BATCH_DATA => {
                if batch[1] & BATCH_ACK != 0 {
                    self.asked = self.batches;
                }
                let mut at = 2;
                while at < batch.len() {
                    let flags = batch[at];
                    let len =
                        u32::from_le_bytes(batch[at + 1..at + 5].try_into().unwrap()) as usize;
                    let bytes = &batch[at + 5..at + 5 + len];
                    at += 5 + len;
                    if flags & DATA_DISCONTINUITY != 0 {
                        self.discontinuities += 1;
                        self.record.clear();
                    }
                    self.record.extend_from_slice(bytes);
                    if flags & DATA_RECORD_END != 0 {
                        let record = std::mem::take(&mut self.record);
                        let unit = self
                            .reader
                            .read(&record)
                            .map(|u| (u.track.0, u.keyframe, u.payload.to_vec()));
                        while let Some(e) = self.reader.poll_event() {
                            match e {
                                ReaderEvent::Configs => self.read.push(None),
                                ReaderEvent::End => self.ends += 1,
                                e => panic!("{e:?}"),
                            }
                        }
                        self.read.extend(unit.map(Some));
                    }
                }
                assert_eq!(at, batch.len(), "messages fill the batch");
            }
            k => panic!("a batch of kind {k}"),
        }
    }

    fn units(&self) -> Vec<&(u32, bool, Vec<u8>)> {
        self.read.iter().flatten().collect()
    }
}

impl Player {
    fn new(port: MessagePort) -> Player {
        let state = Rc::new(RefCell::new(Taken::default()));
        let acking = Rc::new(Cell::new(true));
        let (s, a, p) = (state.clone(), acking.clone(), port.clone());
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let buffer: ArrayBuffer = e.data().dyn_into().unwrap();
            let mut s = s.borrow_mut();
            s.take(&Uint8Array::new(&buffer).to_vec());
            if a.get() && s.asked == s.batches {
                p.post_message(&JsValue::from_f64(s.batches as f64))
                    .unwrap();
            }
        });
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        Player {
            port,
            state,
            acking,
            _on_message: on_message,
        }
    }

    /// Acks every batch taken, and acks from now on.
    fn ack_all(&self) {
        self.acking.set(true);
        let n = self.state.borrow().batches;
        self.port
            .post_message(&JsValue::from_f64(n as f64))
            .unwrap();
    }

    async fn until(&self, what: &str, f: impl Fn(&Taken) -> bool) {
        eventually(what, || async { f(&self.state.borrow()).then_some(()) }).await;
    }

    /// Waits until no batch has come for 300 ms: how many came.
    async fn settled(&self) -> u64 {
        let mut last = self.state.borrow().batches;
        loop {
            sleep(Duration::from_millis(300)).await;
            let now = self.state.borrow().batches;
            if now == last {
                return now;
            }
            last = now;
        }
    }
}

/// The units a capture's appends read back to: track, keyframe, payload.
fn expected(units: &[CaptureUnit]) -> Vec<(u32, bool, Vec<u8>)> {
    units
        .iter()
        .map(|u| (u.track, u.keyframe, u.data.clone()))
        .collect()
}

// --- tests ----------------------------------------------------------------------

#[wasm_bindgen_test]
async fn a_feed_reads_a_capture_on_the_node_and_ends_when_the_capture_closes() {
    let feeds = MessageChannel::new().unwrap();
    let profile = Profile::start("capture-end", feeds.port1()).await;
    let window = Window::new(feeds.port2());
    let id = profile.open_capture().await;
    let listing = profile.listing().await;

    let player = window.open(1, &listing, &profile.network_key, None);
    let media = media_units(2, 1_500);
    profile.append(&id, Vec::new(), media.clone()).await;
    let mut all = first_units();
    all.extend(media);
    let n = all.len();
    player.until("every unit", |t| t.units().len() >= n).await;
    {
        let t = player.state.borrow();
        assert_eq!(t.read.first(), Some(&None), "the configs come first");
        let read: Vec<_> = t.units().into_iter().cloned().collect();
        assert_eq!(read, expected(&all));
        assert_eq!(t.discontinuities, 0);
        // A batch holds what the stream had ready: far fewer than one
        // per record.
        assert!(
            t.batches < n as u64 / 4,
            "{} batches of {n} units",
            t.batches
        );
        assert!(!t.ended);
    }

    profile.close_capture(&id).await;
    player.until("the end batch", |t| t.ended).await;
    assert_eq!(
        player.state.borrow().ends,
        1,
        "the capture's END comes first"
    );
    let closed = eventually("the window hears the feed closed", || async {
        window.closed.borrow().first().cloned()
    })
    .await;
    assert_eq!(closed, (1, "stream closed".to_owned()));
    profile.shut_down().await;
}

#[wasm_bindgen_test]
async fn a_feed_holds_its_stream_while_its_player_does_not_ack_and_a_new_feed_replaces_it() {
    let feeds = MessageChannel::new().unwrap();
    let profile = Profile::start("window", feeds.port1()).await;
    let window = Window::new(feeds.port2());
    let id = profile.open_capture().await;
    let listing = profile.listing().await;

    let slow = window.open(1, &listing, &profile.network_key, None);
    slow.acking.set(false);
    // Two seconds of 128 KiB pictures, more than the window holds.
    let big = media_units(2, 128 * 1024);
    let video: usize = big.iter().map(|u| u.data.len()).sum();
    assert!(video > WINDOW_BYTES + 2 * MAX_BATCH_BYTES);
    for half in big.chunks(big.len() / 2 + 1) {
        profile.append(&id, Vec::new(), half.to_vec()).await;
    }
    // The window closes on its bytes or its batches, whichever comes
    // first, and the stream waits.
    let held = slow.settled().await;
    let bytes = slow.state.borrow().bytes;
    let at_bound = held == WINDOW_BATCHES as u64 || bytes >= WINDOW_BYTES;
    assert!(
        at_bound && held <= WINDOW_BATCHES as u64 && bytes <= WINDOW_BYTES + MAX_BATCH_BYTES + 4096,
        "{bytes} bytes in {held} batches held"
    );
    assert!(bytes < video, "the stream waits for the player");
    slow.ack_all();
    let n = first_units().len() + big.len();
    slow.until("every unit once acked", |t| t.units().len() >= n)
        .await;

    // A new feed holds the swarm before the old one lets go, and reads
    // from the GOP's start.
    let next = window.open(2, &listing, &profile.network_key, Some(1));
    next.until("the new feed's units", |t| t.units().len() >= n)
        .await;
    let taken = slow.settled().await;
    let more = media_units(3, 1_500).split_off(big.len());
    profile.append(&id, Vec::new(), more.clone()).await;
    next.until("the units after", |t| t.units().len() >= n + more.len())
        .await;
    {
        let t = next.state.borrow();
        assert_eq!(
            t.read.first(),
            Some(&None),
            "the new feed starts at the configs"
        );
        let mut all = first_units();
        all.extend(big);
        all.extend(more);
        let read: Vec<_> = t.units().into_iter().cloned().collect();
        assert_eq!(read, expected(&all));
    }
    assert_eq!(
        slow.settled().await,
        taken,
        "the replaced feed posts nothing more"
    );
    assert!(!slow.state.borrow().ended);

    // A feed the window closes posts nothing more, not even its end.
    window.close(2);
    sleep(Duration::from_millis(100)).await;
    let after = next.state.borrow().batches;
    profile
        .append(&id, Vec::new(), vec![unit(VIDEO, 3_500_000, true, 1_500)])
        .await;
    assert_eq!(next.settled().await, after);
    assert!(!next.state.borrow().ended);
    assert!(
        window.closed.borrow().is_empty(),
        "{:?}",
        window.closed.borrow()
    );
    profile.shut_down().await;
}

#[wasm_bindgen_test]
async fn a_feed_that_does_not_open_ends_at_once_and_says_why() {
    let feeds = MessageChannel::new().unwrap();
    let profile = Profile::start("refused", feeds.port1()).await;
    let window = Window::new(feeds.port2());
    let player = window.open(
        7,
        &("magnet:?bad".to_owned(), "video/x.strims-units".to_owned()),
        &profile.network_key,
        None,
    );
    player.until("the end batch", |t| t.ended).await;
    let closed = eventually("the window hears why", || async {
        window.closed.borrow().first().cloned()
    })
    .await;
    assert_eq!(closed.0, 7);
    assert!(!closed.1.is_empty());
    assert_eq!(player.state.borrow().batches, 1, "only the end");
    profile.shut_down().await;
}
