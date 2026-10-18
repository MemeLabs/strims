// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Signing up, in and between devices through the frontend services, over
//! in-memory RPC connections.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    clippy::disallowed_types
)]

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::StreamExt;
use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_dao::auth::ProfileKey;
use strims_dao::schema::network::Networks;
use strims_dao::schema::notification::{dispatch, network_subject, new_notification};
use strims_dao::seal::StorageKey;
use strims_dao::{Error, Table};
use strims_kv::MemoryKv;
use strims_proto::auth::v1::{
    AuthFrontendClient, SignInRequest, SignUpRequest, linked_profile,
    register_auth_frontend_service, sign_in_request,
};
use strims_proto::network::v1::Network;
use strims_proto::notification::v1::event::Body;
use strims_proto::notification::v1::notification::Status;
use strims_proto::notification::v1::{DismissRequest, NotificationFrontendClient, WatchRequest};
use strims_proto::profile::v1::{GetProfileRequest, ListDevicesRequest, ProfileFrontendClient};
use strims_proto::replication::v1::{CreatePairingTokenRequest, ReplicationFrontendClient};
use strims_proto::service::CallError;
use strims_proto::types::Certificate;
use strims_rpc::{Client, Config, Router, Timer, WeakRouter, connect};
use strims_session::{AuthService, Manager, Session, SessionFactory, bind_frontends};
use tokio::task::{JoinHandle, LocalSet, spawn_local};
use tokio::time::{Instant, Sleep};

const T0: Time = Time::from_unix_millis(1_700_000_000_000);

#[derive(Clone)]
struct TokioTimer {
    base: Instant,
}

impl Timer for TokioTimer {
    type Sleep = Sleep;

    fn now(&self) -> Time {
        T0 + Duration::from(self.base.elapsed())
    }

    fn sleep_until(&self, deadline: Time) -> Sleep {
        tokio::time::sleep_until(self.base + (deadline - T0).to_std().unwrap_or_default())
    }
}

/// Counts starts, and can hold them until released.
struct TestFactory {
    timer: TokioTimer,
    starts: Rc<Cell<u32>>,
    hold: Rc<Cell<bool>>,
}

impl SessionFactory<MemoryKv> for TestFactory {
    type Runtime = u32;
    type Error = Error;

    async fn start(&self, _: &Rc<Session<MemoryKv>>) -> Result<u32, Error> {
        while self.hold.get() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        self.starts.set(self.starts.get() + 1);
        Ok(self.starts.get())
    }

    fn bind(&self, _: &u32, session: &Rc<Session<MemoryKv>>, router: &Router) {
        bind_frontends(session, &self.timer, router);
    }
}

type TestManager = Manager<MemoryKv, TestFactory, TokioTimer>;

fn manager(kv: &MemoryKv, timer: &TokioTimer, seed: u64) -> Rc<TestManager> {
    let factory = TestFactory {
        timer: timer.clone(),
        starts: Rc::default(),
        hold: Rc::default(),
    };
    Rc::new(Manager::new(
        kv.clone(),
        factory,
        timer.clone(),
        "test-os",
        det_rng(seed),
    ))
}

// --- pipes ------------------------------------------------------------------

#[derive(Debug)]
struct Closed;

#[derive(Default)]
struct PipeState {
    queue: VecDeque<Vec<u8>>,
    reader: Option<Waker>,
}

#[derive(Clone, Default)]
struct Pipe(Rc<RefCell<PipeState>>);

struct Reader(Pipe);
struct Writer(Pipe);

impl Stream for Reader {
    type Item = Result<Vec<u8>, Closed>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut s = self.0.0.borrow_mut();
        match s.queue.pop_front() {
            Some(b) => Poll::Ready(Some(Ok(b))),
            None => {
                s.reader = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Sink<Vec<u8>> for Writer {
    type Error = Closed;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> Result<(), Closed> {
        let w = {
            let mut s = self.0.0.borrow_mut();
            s.queue.push_back(item);
            s.reader.take()
        };
        if let Some(w) = w {
            w.wake();
        }
        Ok(())
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Result<(), Closed>> {
        Poll::Ready(Ok(()))
    }
}

/// A frontend connection to `manager`, with only the auth service bound
/// until a sign-in. Returns the UI's client.
fn frontend(manager: &Rc<TestManager>, session_key: Option<StorageKey>) -> Client {
    frontend_connection(manager, session_key).client
}

struct FrontendConnection {
    client: Client,
    router: WeakRouter,
    server: JoinHandle<()>,
}

fn frontend_connection(
    manager: &Rc<TestManager>,
    session_key: Option<StorageKey>,
) -> FrontendConnection {
    let (up, down) = (Pipe::default(), Pipe::default());
    let mut router = Router::new();
    let auth = AuthService::new(manager.clone(), session_key, router.clone());
    register_auth_frontend_service(&mut router, Rc::new(auth));
    let weak = router.downgrade();
    let (_, server) = connect(
        Config::frontend(),
        Reader(up.clone()),
        Writer(down.clone()),
        Some(router),
    );
    let (client, conn) = connect(Config::frontend(), Reader(down), Writer(up), None);
    let server = spawn_local(async move {
        let _ = server.await;
    });
    spawn_local(async move {
        let _ = conn.await;
    });
    FrontendConnection {
        client,
        router: weak,
        server,
    }
}

fn run_local<F: Future>(f: F) -> F::Output {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .start_paused(true)
        .build()
        .unwrap();
    rt.block_on(LocalSet::new().run_until(f))
}

fn sign_up(name: &str, persist_login: bool) -> SignUpRequest {
    SignUpRequest {
        name: name.into(),
        password: "pw".into(),
        persist_session: false,
        persist_login,
    }
}

fn password(name: &str, pw: &str, persist_login: bool) -> SignInRequest {
    SignInRequest {
        credentials: Some(sign_in_request::Credentials::Password(
            sign_in_request::Password {
                name: name.into(),
                password: pw.into(),
                persist_login,
                ..Default::default()
            },
        )),
    }
}

#[test]
fn a_closed_connection_frees_its_services() {
    run_local(async {
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let kv = MemoryKv::new();
        let m = manager(&kv, &timer, 1);
        let c = frontend_connection(&m, None);
        let auth = AuthFrontendClient::new(c.client.clone());
        auth.sign_up(&sign_up("alice", false)).await.unwrap();
        let profile = ProfileFrontendClient::new(c.client.clone());
        profile.get(&GetProfileRequest {}).await.unwrap();
        assert!(c.router.upgrade().is_some());

        c.server.abort();
        let _ = c.server.await;
        assert!(c.router.upgrade().is_none());
    });
}

#[test]
fn signing_up_binds_the_profiles_services() {
    run_local(async {
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let kv = MemoryKv::new();
        let m = manager(&kv, &timer, 1);
        let client = frontend(&m, None);
        let profile = ProfileFrontendClient::new(client.clone());
        assert!(matches!(
            profile.get(&GetProfileRequest {}).await,
            Err(CallError::Remote(_))
        ));

        let auth = AuthFrontendClient::new(client.clone());
        assert!(auth.sign_up(&sign_up(" ", false)).await.is_err());
        let res = auth.sign_up(&sign_up("alice", true)).await.unwrap();
        let linked = res.linked_profile.unwrap();
        assert_eq!(linked.name, "alice");
        let Some(linked_profile::Credentials::Key(key)) = linked.credentials else {
            panic!("a connection without a session key gets the profile key");
        };
        assert_eq!(res.profile.as_ref().unwrap().id, key.profile_id);

        let got = profile
            .get(&GetProfileRequest {})
            .await
            .unwrap()
            .profile
            .unwrap();
        assert_eq!(got.name, "alice");
        let devices = profile.list_devices(&ListDevicesRequest {}).await.unwrap();
        let current = devices.current_device.unwrap();
        assert_eq!(current.os, "test-os");
        assert_eq!(current.last_login, T0.unix());
        assert!(devices.devices.is_empty());
        assert!(
            auth.sign_up(&sign_up("alice", false)).await.is_err(),
            "names are unique"
        );

        // The key signs in on another connection.
        let other = AuthFrontendClient::new(frontend(&m, None));
        let req = SignInRequest {
            credentials: Some(sign_in_request::Credentials::Key(sign_in_request::Key {
                profile_id: key.profile_id,
                profile_key: key.profile_key.clone(),
            })),
        };
        assert_eq!(
            other.sign_in(&req).await.unwrap().profile.unwrap().name,
            "alice"
        );
        assert_eq!(m.factory().starts.get(), 1, "one session per profile");
    });
}

/// The UI watches a session's notifications, and dismissing one deletes
/// it.
#[test]
fn a_session_serves_its_notifications() {
    run_local(async {
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let kv = MemoryKv::new();
        let m = manager(&kv, &timer, 1);
        let client = frontend(&m, None);
        let res = AuthFrontendClient::new(client.clone())
            .sign_up(&sign_up("alice", true))
            .await
            .unwrap();
        let Some(linked_profile::Credentials::Key(key)) = res.linked_profile.unwrap().credentials
        else {
            panic!("a connection without a session key gets the profile key");
        };
        let pk = ProfileKey::from_parts(key.profile_id, &key.profile_key).unwrap();
        let (session, _) = m.get_or_create(&pk).await.unwrap();

        let notifications = NotificationFrontendClient::new(client);
        let mut events = notifications.watch(&WatchRequest {}).await.unwrap();
        let n = new_notification(
            1,
            T0,
            Status::Success,
            "Certificate renewed",
            "",
            Some(network_subject(10)),
        );
        let mut tx = session.store.write(timer.now()).await.unwrap();
        dispatch(&mut tx, n.clone()).await.unwrap();
        tx.commit().await.unwrap();
        let mut next = async || {
            let r = events.next().await.unwrap().unwrap();
            r.event.unwrap().body.unwrap()
        };
        assert_eq!(next().await, Body::Notification(n));

        let req = DismissRequest { ids: vec![1] };
        notifications.dismiss(&req).await.unwrap();
        assert_eq!(next().await, Body::Dismiss(1));
    });
}

#[test]
fn passwords_and_session_tokens_sign_in() {
    run_local(async {
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let kv = MemoryKv::new();
        let m = manager(&kv, &timer, 2);
        let session_key = StorageKey::from_bytes([5; 32]);
        let auth = AuthFrontendClient::new(frontend(&m, Some(session_key.clone())));
        auth.sign_up(&sign_up("bob", false)).await.unwrap();

        assert!(
            auth.sign_in(&password("bob", "wrong", false))
                .await
                .is_err()
        );
        assert!(
            auth.sign_in(&password("nobody", "pw", false))
                .await
                .is_err()
        );
        let res = auth.sign_in(&password("bob", "pw", true)).await.unwrap();
        let Some(linked_profile::Credentials::Token(token)) =
            res.linked_profile.unwrap().credentials
        else {
            panic!("a connection with a session key gets a token");
        };
        let by_token = SignInRequest {
            credentials: Some(sign_in_request::Credentials::Token(
                sign_in_request::Token {
                    profile_id: token.profile_id,
                    token: token.token.clone(),
                    eol: token.eol,
                },
            )),
        };
        let fresh = AuthFrontendClient::new(frontend(&m, Some(session_key.clone())));
        assert_eq!(
            fresh
                .sign_in(&by_token)
                .await
                .unwrap()
                .profile
                .unwrap()
                .name,
            "bob"
        );
        let other_key =
            AuthFrontendClient::new(frontend(&m, Some(StorageKey::from_bytes([6; 32]))));
        assert!(other_key.sign_in(&by_token).await.is_err());

        tokio::time::sleep(strims_dao::auth::SESSION_TTL.to_std().unwrap()).await;
        assert!(
            fresh.sign_in(&by_token).await.is_err(),
            "expired tokens are refused"
        );
    });
}

#[test]
fn a_pairing_token_makes_a_new_device() {
    run_local(async {
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let (kv_a, kv_b) = (MemoryKv::new(), MemoryKv::new());
        let (a, b) = (manager(&kv_a, &timer, 3), manager(&kv_b, &timer, 4));
        let client_a = frontend(&a, None);
        let res = AuthFrontendClient::new(client_a.clone())
            .sign_up(&sign_up("carol", true))
            .await
            .unwrap();
        let Some(linked_profile::Credentials::Key(key)) = res.linked_profile.unwrap().credentials
        else {
            panic!();
        };
        let pk = ProfileKey::from_parts(key.profile_id, &key.profile_key).unwrap();

        let replication = ReplicationFrontendClient::new(client_a.clone());
        assert!(
            replication
                .create_pairing_token(&CreatePairingTokenRequest::default())
                .await
                .is_err()
        );
        let (session, _) = a.get_or_create(&pk).await.unwrap();
        let mut tx = session.store.write(timer.now()).await.unwrap();
        let id = tx.generate_id().await.unwrap();
        Networks::insert(
            &mut tx,
            Network {
                id,
                certificate: Some(Certificate {
                    key: vec![1; 32],
                    ..Default::default()
                }),
                ..Default::default()
            },
        )
        .await
        .unwrap();
        tx.commit().await.unwrap();
        let token = replication
            .create_pairing_token(&CreatePairingTokenRequest::default())
            .await
            .unwrap()
            .token
            .unwrap();
        assert!(token.networks.iter().all(|n| n.server_config.is_none()));
        let new_device = token.profile.as_ref().unwrap().device_id;

        let client_b = frontend(&b, None);
        let mut req = password("", "pw", false);
        if let Some(sign_in_request::Credentials::Password(p)) = &mut req.credentials {
            p.pairing_token = Some(token.clone());
        }
        let res = AuthFrontendClient::new(client_b.clone())
            .sign_in(&req)
            .await
            .unwrap();
        assert_eq!(res.profile.unwrap().device_id, new_device);
        let devices = ProfileFrontendClient::new(client_b)
            .list_devices(&ListDevicesRequest {})
            .await
            .unwrap();
        assert_eq!(devices.current_device.unwrap().id, new_device);
        assert_eq!(devices.devices.len(), 1);

        let mut wrong = req.clone();
        if let Some(sign_in_request::Credentials::Password(p)) = &mut wrong.credentials {
            p.password = "nope".into();
        }
        let fresh = AuthFrontendClient::new(frontend(&manager(&MemoryKv::new(), &timer, 5), None));
        assert!(fresh.sign_in(&wrong).await.is_err());
    });
}

#[test]
fn concurrent_sign_ins_share_one_start() {
    run_local(async {
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let kv = MemoryKv::new();
        let m = manager(&kv, &timer, 6);
        let mut rng = det_rng(9);
        let (pk, _) = strims_dao::auth::sign_up(&kv, "dave", "pw", "os", &mut rng, timer.now())
            .await
            .unwrap();

        m.factory().hold.set(true);
        let first = spawn_local({
            let (m, pk) = (m.clone(), pk.clone());
            async move { m.get_or_create(&pk).await.map(|(_, r)| *r) }
        });
        let second = spawn_local({
            let (m, pk) = (m.clone(), pk.clone());
            async move { m.get_or_create(&pk).await.map(|(_, r)| *r) }
        });
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        m.factory().hold.set(false);
        assert_eq!(first.await.unwrap().unwrap(), 1);
        assert_eq!(second.await.unwrap().unwrap(), 1);

        let wrong = ProfileKey {
            id: pk.id,
            key: [0; 32],
        };
        assert!(matches!(
            m.get_or_create(&wrong).await,
            Err(Error::Credentials)
        ));
    });
}

#[test]
fn a_cancelled_start_lets_waiters_start_again() {
    run_local(async {
        let timer = TokioTimer {
            base: Instant::now(),
        };
        let kv = MemoryKv::new();
        let m = manager(&kv, &timer, 7);
        let mut rng = det_rng(9);
        let (pk, _) = strims_dao::auth::sign_up(&kv, "erin", "pw", "os", &mut rng, timer.now())
            .await
            .unwrap();

        m.factory().hold.set(true);
        let cancelled = spawn_local({
            let (m, pk) = (m.clone(), pk.clone());
            async move { m.get_or_create(&pk).await.map(|(_, r)| *r) }
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let waiter = spawn_local({
            let (m, pk) = (m.clone(), pk.clone());
            async move { m.get_or_create(&pk).await.map(|(_, r)| *r) }
        });
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        cancelled.abort();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        m.factory().hold.set(false);
        assert_eq!(waiter.await.unwrap().unwrap(), 1);
    });
}
