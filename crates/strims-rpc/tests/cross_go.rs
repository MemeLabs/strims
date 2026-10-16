// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Cross-implementation runs against the Go RPC library: the Go peer in
//! `crates/strims-rpc/golden` serves its test Debug service to a Rust
//! client, and checks a Rust server of the same service as a client. The
//! peers speak over the Go process's stdin and stdout. Skipped when `go`
//! isn't on PATH.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing,
    // These tests drive a real subprocess, so they need threads.
    clippy::disallowed_methods
)]

use std::cell::Cell;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::pin::Pin;
use std::process::{Child, Command, Stdio};
use std::rc::Rc;
use std::sync::mpsc;
use std::task::{Context, Poll};
use std::thread::JoinHandle;
use std::time::Duration;

use futures_core::Stream;
use futures_sink::Sink;
use futures_util::{StreamExt, stream};
use strims_proto::debug::v1::{
    DebugClient, DebugService, GetConfigRequest, GetConfigResponse, PProfRequest, PProfResponse,
    ReadMetricsRequest, ReadMetricsResponse, SetConfigRequest, StartMockStreamRequest,
    StartMockStreamResponse, WatchMetricsRequest, WatchMetricsResponse, register_debug_service,
};
use strims_proto::rpc::call::Kind;
use strims_proto::service::{
    CallError, Caller, Context as CallContext, Error, Method, ServerStream, UnaryMethod,
};
use strims_rpc::wire::{call_len, uvarint_len};
use strims_rpc::{Config, Router, connect};
use tokio::sync::mpsc as tokio_mpsc;
use tokio::task::{LocalSet, spawn_local};

const PPROF: &str = "strims.debug.v1.Debug.PProf";

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn go_available() -> bool {
    match Command::new("go").arg("version").output() {
        Ok(o) if o.status.success() => true,
        _ => {
            eprintln!("skipping: `go` is not on PATH");
            false
        }
    }
}

/// The Go process's stdout as a stream of byte chunks.
struct GoReader(tokio_mpsc::UnboundedReceiver<Vec<u8>>);

impl Stream for GoReader {
    type Item = io::Result<Vec<u8>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx).map(|b| b.map(Ok))
    }
}

/// The Go process's stdin as a sink of Calls.
struct GoWriter(mpsc::Sender<Vec<u8>>);

impl Sink<Vec<u8>> for GoWriter {
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Vec<u8>) -> io::Result<()> {
        self.0
            .send(item)
            .map_err(|_| io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

struct GoPeer {
    child: Child,
    stderr: JoinHandle<String>,
}

impl GoPeer {
    /// Waits for the process and returns its stderr if it failed.
    fn finish(mut self) -> Result<(), String> {
        let status = self.child.wait().expect("wait for go");
        let stderr = self.stderr.join().unwrap();
        if status.success() {
            Ok(())
        } else {
            Err(format!("go peer exited with {status}:\n{stderr}"))
        }
    }
}

/// Starts the Go peer in `mode`, with threads pumping its stdio. Dropping
/// the writer closes its stdin.
fn spawn_go(mode: &str) -> (GoPeer, GoReader, GoWriter) {
    let mut child = Command::new("go")
        .args(["run", "-tags", "golden", "./crates/strims-rpc/golden", mode])
        .current_dir(repo_root())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn go");

    let mut stdout = child.stdout.take().unwrap();
    let (in_tx, in_rx) = tokio_mpsc::unbounded_channel();
    std::thread::spawn(move || {
        let mut buf = vec![0u8; 16 * 1024];
        loop {
            match stdout.read(&mut buf) {
                Ok(0) | Err(_) => return,
                Ok(n) => {
                    if in_tx.send(buf[..n].to_vec()).is_err() {
                        return;
                    }
                }
            }
        }
    });

    let mut stdin = child.stdin.take().unwrap();
    let (out_tx, out_rx) = mpsc::channel::<Vec<u8>>();
    std::thread::spawn(move || {
        for b in out_rx {
            if stdin.write_all(&b).is_err() {
                return;
            }
        }
    });

    let mut err = child.stderr.take().unwrap();
    let stderr = std::thread::spawn(move || {
        let mut s = String::new();
        let _ = err.read_to_string(&mut s);
        s
    });

    (GoPeer { child, stderr }, GoReader(in_rx), GoWriter(out_tx))
}

/// Byte `i` of a test payload.
fn data(n: usize) -> Vec<u8> {
    (0..n).map(|i| (i * 7) as u8).collect()
}

fn pprof(name: impl Into<String>) -> PProfRequest {
    PProfRequest {
        name: name.into(),
        ..Default::default()
    }
}

fn watch(interval_ms: i32) -> WatchMetricsRequest {
    WatchMetricsRequest {
        interval_ms,
        ..Default::default()
    }
}

/// A method the test service doesn't have.
struct Nope;

impl Method for Nope {
    const NAME: &'static str = "strims.debug.v1.Debug.Nope";
    type Request = PProfRequest;
    type Response = PProfResponse;
}

impl UnaryMethod for Nope {}

/// Waits until the server has seen `n` cancels.
async fn wait_cancelled<C: Caller>(debug: &DebugClient<C>, n: u64) {
    for _ in 0..500 {
        let res = debug
            .start_mock_stream(&StartMockStreamRequest::default())
            .await
            .unwrap();
        if res.id >= n {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("the server saw fewer than {n} cancels");
}

#[tokio::test]
async fn rust_client_against_go_server() {
    if !go_available() {
        return;
    }
    let (go, reader, writer) = spawn_go("server");
    LocalSet::new()
        .run_until(async move {
            let (client, conn) = connect(Config::peer(), reader, writer, None);
            let conn = spawn_local(conn);
            let debug = DebugClient::new(client.clone());

            for n in [0, 1000, 500_000] {
                let res = debug.p_prof(&pprof(n.to_string())).await.unwrap();
                assert_eq!(res.name, n.to_string());
                assert!(res.data == data(n), "pprof {n}");
            }

            // a request of exactly the 512 KiB limit; ids stay under 128,
            // so their varints take one byte
            let max = Config::peer().max_send_bytes;
            let name_len = (0..max)
                .rev()
                .find(|&n| {
                    let arg = 1 + uvarint_len(n as u64) + n;
                    call_len(1, 0, PPROF, Kind::CallKindDefault, arg) == max
                })
                .unwrap();
            let res = debug.p_prof(&pprof("x".repeat(name_len))).await.unwrap();
            assert_eq!(res.name, name_len.to_string());

            let items: Vec<_> = debug
                .watch_metrics(&watch(5))
                .await
                .unwrap()
                .collect()
                .await;
            let items: Vec<Vec<u8>> = items.into_iter().map(|i| i.unwrap().data).collect();
            assert_eq!(items, [[0], [1], [2], [3], [4]]);

            // dropping a stream cancels it at the server
            let mut endless = debug.watch_metrics(&watch(0)).await.unwrap();
            for _ in 0..3 {
                endless.next().await.unwrap().unwrap();
            }
            drop(endless);
            wait_cancelled(&debug, 1).await;

            // so does a caller's deadline
            let read = debug.read_metrics(&ReadMetricsRequest::default());
            assert!(
                tokio::time::timeout(Duration::from_millis(200), read)
                    .await
                    .is_err()
            );
            wait_cancelled(&debug, 2).await;

            let mut negative = debug.watch_metrics(&watch(-1)).await.unwrap();
            assert_eq!(
                negative.next().await,
                Some(Err(CallError::Remote(Error::new(3, "negative interval"))))
            );
            assert_eq!(negative.next().await, None);

            assert_eq!(
                debug.get_config(&GetConfigRequest::default()).await,
                Err(CallError::Remote(Error::new(7, "no config")))
            );
            assert_eq!(
                debug.set_config(&SetConfigRequest::default()).await,
                Err(CallError::Remote(Error::new(0, "not implemented")))
            );
            assert_eq!(
                client.call_unary::<Nope>(&pprof("")).await,
                Err(CallError::Remote(Error::method_not_found(Nope::NAME)))
            );

            // closing our end ends the Go server
            conn.abort();
        })
        .await;
    go.finish().unwrap();
}

/// The Go peer's test service, in Rust.
#[derive(Default)]
struct TestService {
    cancelled: Rc<Cell<u64>>,
}

/// Counts a cancel when the call it's part of is dropped.
struct CancelGuard(Rc<Cell<u64>>);

impl Drop for CancelGuard {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}

impl DebugService for TestService {
    async fn p_prof(&self, _: &CallContext, req: PProfRequest) -> Result<PProfResponse, Error> {
        let name = if req.name.len() > 64 {
            req.name.len().to_string()
        } else {
            req.name.clone()
        };
        Ok(PProfResponse {
            name,
            data: data(req.name.parse().unwrap_or(0)),
        })
    }

    async fn read_metrics(
        &self,
        _: &CallContext,
        _: ReadMetricsRequest,
    ) -> Result<ReadMetricsResponse, Error> {
        let _guard = CancelGuard(self.cancelled.clone());
        std::future::pending().await
    }

    async fn watch_metrics(
        &self,
        _: &CallContext,
        req: WatchMetricsRequest,
    ) -> Result<ServerStream<WatchMetricsResponse>, Error> {
        let item = |i: i32| WatchMetricsResponse {
            data: vec![i as u8],
        };
        match req.interval_ms {
            n if n < 0 => Err(Error::new(3, "negative interval")),
            0 => {
                let guard = CancelGuard(self.cancelled.clone());
                Ok(Box::pin(stream::unfold(
                    (guard, 0),
                    move |(g, i)| async move {
                        tokio::time::sleep(Duration::from_millis(1)).await;
                        Some((Ok(item(i)), (g, i + 1)))
                    },
                )))
            }
            n => Ok(Box::pin(stream::iter((0..n).map(move |i| Ok(item(i)))))),
        }
    }

    async fn get_config(
        &self,
        _: &CallContext,
        _: GetConfigRequest,
    ) -> Result<GetConfigResponse, Error> {
        Err(Error::new(7, "no config"))
    }

    async fn start_mock_stream(
        &self,
        _: &CallContext,
        _: StartMockStreamRequest,
    ) -> Result<StartMockStreamResponse, Error> {
        Ok(StartMockStreamResponse {
            id: self.cancelled.get(),
        })
    }
}

#[tokio::test]
async fn go_client_against_rust_server() {
    if !go_available() {
        return;
    }
    let (go, reader, writer) = spawn_go("client");
    LocalSet::new()
        .run_until(async move {
            let mut router = Router::new();
            register_debug_service(&mut router, Rc::new(TestService::default()));
            let (_, conn) = connect(Config::peer(), reader, writer, Some(router));
            // the Go client exits once it's done, ending our input
            tokio::time::timeout(Duration::from_secs(300), conn)
                .await
                .expect("go client timed out")
                .unwrap();
        })
        .await;
    go.finish().unwrap();
}
