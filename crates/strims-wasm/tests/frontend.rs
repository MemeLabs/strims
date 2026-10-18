// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The frontend connection in a dedicated worker: a strims-rpc client at
//! the other end of a `MessageChannel`, as the UI's, against
//! `frontend::serve` on a `Manager` of `WasmFactory` and `IdbKv`.
//!
//! Each test's databases share a prefix of their own, and the test
//! deletes them at its end. Every message the worker posts is checked to
//! hold exactly one whole Call.

#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::Cell;
use std::collections::BTreeSet;
use std::future::Future;
use std::rc::Rc;

use futures_util::future::join_all;
use futures_util::{SinkExt, StreamExt};
use js_sys::{Array, Function, Promise, Reflect};
use prost::Message;
use rand_core::SeedableRng;
use strims_base::rng::DetRng;
use strims_base::time::Duration;
use strims_kv::Kv;
use strims_proto::auth::v1::{
    AuthFrontendClient, SignInRequest, SignUpRequest, auth_frontend_methods, linked_profile,
    sign_in_request,
};
use strims_proto::network::v1::bootstrap::{BootstrapFrontendClient, GetConfigRequest};
use strims_proto::network::v1::{ListNetworksRequest, NetworkFrontendClient};
use strims_proto::rpc::Call;
use strims_proto::rpc::call::Kind;
use strims_proto::service::Method;
use strims_rpc::wire::{CALLBACK, encode_call, get_uvarint};
use strims_rpc::{Client, Config as RpcConfig, Framing, Timer, connect};
use strims_wasm::IdbKv;
use strims_wasm::clock::{self, WasmClock};
use strims_wasm::entry::{DEVICE_OS, WasmManager};
use strims_wasm::factory::WasmFactory;
use strims_wasm::options::Options;
use strims_wasm::runtime::{self, TaskHandle};
use strims_wasm::{entropy, frontend};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;
use web_sys::MessageChannel;

wasm_bindgen_test_configure!(run_in_dedicated_worker);

/// A database-name prefix no other test or run shares.
fn unique(test: &str) -> String {
    let seed = entropy::seed().unwrap();
    let tag: String = seed[..6].iter().map(|b| format!("{b:02x}")).collect();
    format!("strims-wasm-test/frontend/{test}/{tag}/")
}

/// The names of every database, from `indexedDB.databases()`.
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

/// Deletes every database whose name starts with `prefix`.
async fn clean_up(prefix: &str) {
    let kv = IdbKv::new("");
    for name in database_names().await {
        if name.starts_with(prefix) {
            kv.delete_table(&name).await.unwrap();
        }
    }
    assert!(
        !database_names().await.iter().any(|n| n.starts_with(prefix)),
        "{prefix} databases remain"
    );
}

async fn sleep(d: Duration) {
    WasmClock.sleep_until(clock::now() + d).await;
}

/// `f`, or a panic naming `what` after `d`.
async fn within<F: Future>(d: Duration, what: &str, f: F) -> F::Output {
    let deadline = WasmClock.sleep_until(clock::now() + d);
    tokio::select! {
        biased;
        v = f => v,
        () = deadline => panic!("timed out waiting for {what}"),
    }
}

/// A manager of the profiles under `prefix`, as `entry::start` builds it,
/// with no ICE servers and a signal port nobody answers.
fn manager(prefix: &str) -> Rc<WasmManager> {
    let options = Options {
        ice_servers: Some(Vec::new()),
        ..Options::default()
    };
    let signal = MessageChannel::new().unwrap().port1();
    Rc::new(WasmManager::new(
        IdbKv::new(prefix),
        WasmFactory::new(&options, signal, None),
        WasmClock,
        DEVICE_OS,
        DetRng::from_seed(entropy::seed().unwrap()),
    ))
}

/// The Calls in a message from the worker, each checked to be whole.
fn calls(message: &[u8]) -> Result<Vec<Call>, String> {
    let mut rest = message;
    let mut out = Vec::new();
    while !rest.is_empty() {
        let (len, n) = get_uvarint(rest)
            .map_err(|e| e.to_string())?
            .ok_or("a length prefix cut short")?;
        let end = usize::try_from(len)
            .ok()
            .and_then(|l| l.checked_add(n))
            .filter(|end| *end <= rest.len())
            .ok_or("a Call cut short")?;
        out.push(Call::decode(&rest[n..end]).map_err(|e| e.to_string())?);
        rest = &rest[end..];
    }
    Ok(out)
}

/// What the UI side saw of the worker's messages.
#[derive(Default)]
struct Seen {
    messages: Cell<usize>,
    /// Messages that weren't exactly one whole Call.
    bad: Cell<usize>,
}

/// The UI's end of a frontend connection, served until it drops.
struct Ui {
    client: Client,
    seen: Rc<Seen>,
    serve: TaskHandle,
    conn: TaskHandle,
}

impl Ui {
    /// Asserts every message so far was one whole Call, and returns how
    /// many there were.
    fn whole_messages(&self) -> usize {
        assert_eq!(
            self.seen.bad.get(),
            0,
            "messages that aren't one whole Call"
        );
        self.seen.messages.get()
    }

    fn auth(&self) -> AuthFrontendClient<Client> {
        AuthFrontendClient::new(self.client.clone())
    }
}

impl Drop for Ui {
    fn drop(&mut self) {
        self.serve.abort();
        self.conn.abort();
    }
}

/// Serves `manager`'s frontend on one port of a channel, and connects a
/// client on the other.
fn connect_ui(manager: &Rc<WasmManager>) -> Ui {
    let channel = MessageChannel::new().unwrap();
    let serve = runtime::spawn({
        let manager = manager.clone();
        let port = channel.port1();
        async move {
            let _ = frontend::serve(manager, port).await;
        }
    });
    let (reader, writer) = frontend::transport(channel.port2());
    let seen = Rc::new(Seen::default());
    let reader = reader.inspect({
        let seen = seen.clone();
        move |m| {
            if let Ok(m) = m {
                seen.messages.set(seen.messages.get() + 1);
                if calls(m).map_or(true, |c| c.len() != 1) {
                    seen.bad.set(seen.bad.get() + 1);
                }
            }
        }
    });
    let (client, conn) = connect(RpcConfig::frontend(), reader, writer, None);
    let conn = runtime::spawn(async move {
        let _ = conn.await;
    });
    Ui {
        client,
        seen,
        serve,
        conn,
    }
}

/// Stops `manager`'s connections and profiles, then deletes `prefix`'s
/// databases.
async fn shut_down(ui: Ui, manager: Rc<WasmManager>, prefix: &str) {
    drop(ui);
    drop(manager);
    // The aborted tasks drop, and the nodes stop.
    sleep(Duration::from_millis(100)).await;
    clean_up(prefix).await;
}

fn sign_up(name: &str, persist_login: bool) -> SignUpRequest {
    SignUpRequest {
        name: name.to_owned(),
        password: "pw".to_owned(),
        persist_session: false,
        persist_login,
    }
}

#[wasm_bindgen_test]
async fn a_sign_up_starts_the_profiles_node_and_binds_its_frontends() {
    let prefix = unique("sign-up");
    let manager = manager(&prefix);
    let ui = connect_ui(&manager);

    // Before a sign-in, only the auth service answers.
    let networks = NetworkFrontendClient::new(ui.client.clone());
    assert!(networks.list(&ListNetworksRequest {}).await.is_err());
    assert!(manager.factory().running().is_none());

    let started = clock::now();
    let res = within(
        Duration::from_secs(60),
        "the sign-up",
        ui.auth().sign_up(&sign_up("alice", false)),
    )
    .await
    .unwrap();
    let took = clock::now() - started;
    console_log!("SignUp took {} ms", took.as_millis());
    assert_eq!(res.profile.unwrap().name, "alice");

    let info = manager
        .factory()
        .running()
        .expect("the profile's node runs");
    assert_eq!((info.tcp_uri, info.ws_uri), (None, None));
    assert!(networks.list(&ListNetworksRequest {}).await.is_ok());
    let config = BootstrapFrontendClient::new(ui.client.clone())
        .get_config(&GetConfigRequest {})
        .await
        .unwrap();
    assert!(config.config.is_some());
    // One answer each to the two lists, the sign-up and the config.
    assert_eq!(ui.whole_messages(), 4);
    shut_down(ui, manager, &prefix).await;
}

#[wasm_bindgen_test]
async fn a_sign_in_with_the_profile_key_opens_the_profile_in_a_new_manager() {
    let prefix = unique("sign-in");
    let first = manager(&prefix);
    let ui = connect_ui(&first);
    let res = ui.auth().sign_up(&sign_up("bob", true)).await.unwrap();
    let profile = res.profile.unwrap();
    let Some(linked_profile::Credentials::Key(key)) = res.linked_profile.unwrap().credentials
    else {
        panic!("a connection without a session key gets the profile key");
    };
    assert_eq!(key.profile_id, profile.id);
    ui.whole_messages();
    drop(ui);
    drop(first);
    sleep(Duration::from_millis(100)).await;

    let second = manager(&prefix);
    let ui = connect_ui(&second);
    let sign_in = SignInRequest {
        credentials: Some(sign_in_request::Credentials::Key(sign_in_request::Key {
            profile_id: key.profile_id,
            profile_key: key.profile_key,
        })),
    };
    let again = ui.auth().sign_in(&sign_in).await.unwrap().profile.unwrap();
    assert_eq!(
        (again.id, &again.name, &again.key),
        (profile.id, &profile.name, &profile.key)
    );
    assert!(second.factory().running().is_some());
    let networks = NetworkFrontendClient::new(ui.client.clone());
    assert!(networks.list(&ListNetworksRequest {}).await.is_ok());

    // A key that doesn't open the profile's store is refused.
    let mut wrong = sign_in.clone();
    if let Some(sign_in_request::Credentials::Key(k)) = &mut wrong.credentials {
        k.profile_key[0] ^= 1;
    }
    assert!(ui.auth().sign_in(&wrong).await.is_err());
    ui.whole_messages();
    shut_down(ui, second, &prefix).await;
}

#[wasm_bindgen_test]
async fn two_calls_in_one_message_are_both_answered() {
    let prefix = unique("two-calls");
    let manager = manager(&prefix);
    let channel = MessageChannel::new().unwrap();
    let serve = runtime::spawn({
        let manager = manager.clone();
        let port = channel.port1();
        async move {
            let _ = frontend::serve(manager, port).await;
        }
    });
    let (mut reader, mut writer) = frontend::transport(channel.port2());

    let mut message = Vec::new();
    for id in [1, 2] {
        encode_call(
            &mut message,
            Framing::Delimited,
            id,
            0,
            auth_frontend_methods::SignIn::NAME,
            Kind::CallKindDefault,
            &SignInRequest::default(),
        );
    }
    writer.send(message).await.unwrap();

    let mut answered = BTreeSet::new();
    while answered.len() < 2 {
        let m = within(Duration::from_secs(5), "an answer", reader.next())
            .await
            .expect("the port ended")
            .unwrap();
        let calls = calls(&m).unwrap();
        assert_eq!(calls.len(), 1, "one Call per message");
        for call in calls {
            assert_eq!(call.method, CALLBACK);
            assert_eq!(call.kind, Kind::CallKindError as i32);
            assert!(answered.insert(call.parent_id), "answered twice");
        }
    }
    assert_eq!(answered, BTreeSet::from([1, 2]));
    serve.abort();
    drop(manager);
    clean_up(&prefix).await;
}

#[wasm_bindgen_test]
async fn every_message_from_the_worker_is_one_whole_call() {
    let prefix = unique("whole");
    let manager = manager(&prefix);
    let ui = connect_ui(&manager);
    let auth = ui.auth();
    let missing = SignInRequest::default();
    let answers = within(
        Duration::from_secs(5),
        "the answers",
        join_all((0..32).map(|_| auth.sign_in(&missing))),
    )
    .await;
    assert!(answers.iter().all(Result::is_err));
    assert_eq!(ui.whole_messages(), 32);
    shut_down(ui, manager, &prefix).await;
}
