// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Live interop with a real browser: headless Firefox runs a page that does
//! what the wasm build's JS bridge does (`RTCPeerConnection`, the
//! pre-negotiated channel `{negotiated: true, id: 1}`, non-trickle
//! offer/answer as `JSON.stringify(pc.localDescription)`), against a Rust
//! [`WebRtcEndpoint`] on a local UDP socket. 2 MiB each way, byte-exact,
//! then a close from Rust.
//!
//! The page keeps Firefox's default mDNS obfuscation of host candidates
//! (`<uuid>.local`, which str0m can't resolve), so the link must come up
//! the way a browser link does in production: through the browser's checks
//! of our host candidate, whose source we learn as peer-reflexive. Our
//! socket must be on the outbound interface: loopback candidates go
//! unchecked in this mode.
//!
//! Ignored by default (it launches a browser):
//!
//! ```sh
//! cargo test -p strims-webrtc --test browser -- --ignored
//! ```
//!
//! Skipped when `firefox` isn't on PATH. A snap Firefox can't see `/tmp`,
//! so the profile goes under `~/snap/firefox/common` when that exists.

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    // A browser, an HTTP server, sockets and the wall clock.
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::time::{Duration as StdDuration, Instant as StdInstant, SystemTime};

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_webrtc::{
    CloseReason, ConnId, DATA_CHANNEL_MTU, EndpointConfig, Event, SdpType, SessionDescription,
    WebRtcEndpoint,
};

const SIZE: usize = 2 << 20;
const GO_STREAM_SEED: u64 = 0x60;
const RUST_STREAM_SEED: u64 = 0x52;

fn now() -> Time {
    Time::from_system(SystemTime::now())
}

fn stream(seed: u64, n: usize) -> Vec<u8> {
    let mut state = seed;
    let mut out = Vec::with_capacity(n + 8);
    while out.len() < n {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        out.extend_from_slice(&(z ^ (z >> 31)).to_le_bytes());
    }
    out.truncate(n);
    out
}

const PAGE: &str = r#"<!doctype html><meta charset="utf-8"><title>strims-webrtc</title>
<script>
const q = new URLSearchParams(location.search);
const role = q.get('role');
const size = Number(q.get('size'));
const post = (path, body) => fetch(path, { method: 'POST', body });
const log = (m) => post('/log', m);
function stream(seed, n) {
  const out = new Uint8Array(Math.ceil(n / 8) * 8);
  const dv = new DataView(out.buffer);
  const M = (1n << 64n) - 1n;
  let s = BigInt(seed);
  for (let i = 0; i < out.length; i += 8) {
    s = (s + 0x9E3779B97F4A7C15n) & M;
    let z = s;
    z = ((z ^ (z >> 30n)) * 0xBF58476D1CE4E5B9n) & M;
    z = ((z ^ (z >> 27n)) * 0x94D049BB133111EBn) & M;
    dv.setBigUint64(i, z ^ (z >> 31n), true);
  }
  return out.subarray(0, n);
}
async function main() {
  const pc = new RTCPeerConnection({ iceServers: [] });
  const dc = pc.createDataChannel('data', { negotiated: true, id: 1 });
  dc.binaryType = 'arraybuffer';
  const opened = new Promise((r) => dc.addEventListener('open', r));
  const closed = new Promise((r) => dc.addEventListener('close', r));
  // Listen before anything is awaited: messages dispatched before a
  // listener exists are gone.
  const want = stream(0x52, size);
  let got = 0, largest = 0, bad = -1;
  const received = new Promise((r) => {
    dc.addEventListener('message', (e) => {
      const b = new Uint8Array(e.data);
      largest = Math.max(largest, b.length);
      for (let i = 0; i < b.length && bad < 0; i++) {
        if (got + i >= size || b[i] !== want[got + i]) bad = got + i;
      }
      got += b.length;
      if (got >= size) r();
    });
  });
  const gathered = () => new Promise((r) => {
    if (pc.iceGatheringState === 'complete') return r();
    pc.addEventListener('icegatheringstatechange', () => {
      if (pc.iceGatheringState === 'complete') r();
    });
  });
  const sendDesc = () => post('/desc', JSON.stringify(pc.localDescription));
  const recvDesc = async () => JSON.parse(await (await fetch('/desc')).text());
  if (role === 'offer') {
    await pc.setLocalDescription(await pc.createOffer());
    await gathered();
    await sendDesc();
    await pc.setRemoteDescription(await recvDesc());
  } else {
    await pc.setRemoteDescription(await recvDesc());
    await pc.setLocalDescription(await pc.createAnswer());
    await gathered();
    await sendDesc();
  }
  await opened;
  await log('open ' + pc.sctp.maxMessageSize);
  const out = stream(0x60, size);
  dc.bufferedAmountLowThreshold = 1 << 20;
  for (let off = 0; off < size; off += 65536) {
    if (dc.bufferedAmount > (4 << 20)) {
      await new Promise((r) => dc.addEventListener('bufferedamountlow', r, { once: true }));
    }
    dc.send(out.subarray(off, Math.min(size, off + 65536)));
  }
  await received;
  await log(bad < 0 ? `recv-ok ${got} ${largest}` : `recv-bad ${bad}`);
  await closed;
  await log('closed ' + pc.connectionState);
}
main().catch((e) => log('error ' + e));
</script>"#;

/// What the page sends us.
enum FromPage {
    Desc(String),
    Log(String),
}

/// A minimal HTTP/1.1 server for the page and its signalling.
fn serve(to_test: Sender<FromPage>, to_page: Receiver<String>) -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let to_page = Arc::new(Mutex::new(to_page));
    std::thread::spawn(move || {
        for conn in listener.incoming() {
            let Ok(conn) = conn else { return };
            let to_test = to_test.clone();
            let to_page = to_page.clone();
            std::thread::spawn(move || handle(conn, &to_test, &to_page));
        }
    });
    addr
}

fn handle(mut conn: TcpStream, to_test: &Sender<FromPage>, to_page: &Mutex<Receiver<String>>) {
    let mut reader = BufReader::new(conn.try_clone().unwrap());
    let mut request = String::new();
    if reader.read_line(&mut request).is_err() {
        return;
    }
    let mut len = 0;
    loop {
        let mut h = String::new();
        if reader.read_line(&mut h).is_err() || h.trim().is_empty() {
            break;
        }
        if let Some((k, v)) = h.split_once(':')
            && k.eq_ignore_ascii_case("content-length")
        {
            len = v.trim().parse().unwrap_or(0);
        }
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).unwrap();
    let body = String::from_utf8(body).unwrap();
    let mut parts = request.split_whitespace();
    let (method, path) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let (status, ctype, reply) = match (method, path.split('?').next().unwrap_or("")) {
        ("GET", "/") => ("200 OK", "text/html", PAGE.to_owned()),
        ("POST", "/desc") => {
            let _ = to_test.send(FromPage::Desc(body));
            ("200 OK", "text/plain", String::new())
        }
        ("GET", "/desc") => {
            let d = to_page
                .lock()
                .unwrap()
                .recv_timeout(StdDuration::from_secs(60))
                .unwrap_or_default();
            ("200 OK", "application/json", d)
        }
        ("POST", "/log") => {
            let _ = to_test.send(FromPage::Log(body));
            ("200 OK", "text/plain", String::new())
        }
        _ => ("404 Not Found", "text/plain", String::new()),
    };
    let _ = write!(
        conn,
        "HTTP/1.1 {status}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
        reply.len()
    );
}

fn firefox_profile() -> Option<PathBuf> {
    let found = Command::new("firefox")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if !found {
        eprintln!("skipping: `firefox` is not on PATH");
        return None;
    }
    // A snap Firefox has a private /tmp; its own common dir is visible.
    let base = std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join("snap/firefox/common"))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_TARGET_TMPDIR")));
    let profile = base.join(format!("strims-webrtc-test-{}", std::process::id()));
    std::fs::create_dir_all(&profile).unwrap();
    std::fs::write(
        profile.join("user.js"),
        concat!(
            "user_pref(\"browser.shell.checkDefaultBrowser\", false);\n",
            "user_pref(\"datareporting.policy.dataSubmissionEnabled\", false);\n",
        ),
    )
    .unwrap();
    Some(profile)
}

/// The address of the interface with the default route (no datagram is
/// sent), or loopback.
fn outbound_ip() -> std::net::IpAddr {
    let probe = UdpSocket::bind("0.0.0.0:0").unwrap();
    match probe
        .connect("192.0.2.1:9")
        .and_then(|()| probe.local_addr())
    {
        Ok(a) if !a.ip().is_unspecified() => a.ip(),
        _ => std::net::Ipv4Addr::LOCALHOST.into(),
    }
}

struct Browser {
    child: Child,
    profile: PathBuf,
}

impl Drop for Browser {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.profile);
    }
}

fn run(rust_offers: bool) {
    let Some(profile) = firefox_profile() else {
        return;
    };
    let (to_test, from_page) = mpsc::channel();
    let (to_page, page_rx) = mpsc::channel();
    let http = serve(to_test, page_rx);
    let role = if rust_offers { "answer" } else { "offer" };
    let url = format!("http://{http}/?role={role}&size={SIZE}");
    let child = Command::new("firefox")
        .args(["--headless", "--no-remote", "--profile"])
        .arg(&profile)
        .arg(&url)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _browser = Browser { child, profile };

    // Must not be loopback: see the module docs.
    let sock = UdpSocket::bind((outbound_ip(), 0)).unwrap();
    let local = sock.local_addr().unwrap();
    let config = EndpointConfig {
        ice_servers: Some(vec![]),
        ..EndpointConfig::default()
    };
    let mut ep = WebRtcEndpoint::new(now(), config, det_rng(5));
    ep.add_local_addr(local);
    let mut buf = vec![0u8; 65536];
    let mut log: Vec<String> = vec![];
    let mut pump = |ep: &mut WebRtcEndpoint, log: &mut Vec<String>| -> Vec<Event> {
        while let Some(t) = ep.poll_udp_transmit() {
            sock.send_to(&t.contents, t.destination).unwrap();
        }
        let n = now();
        let wait = ep
            .poll_timeout()
            .map_or(Duration::from_millis(5), |t| t - n)
            .clamp(Duration::from_micros(100), Duration::from_millis(5));
        sock.set_read_timeout(wait.to_std()).unwrap();
        for _ in 0..256 {
            match sock.recv_from(&mut buf) {
                Ok((len, src)) => {
                    ep.handle_udp_input(now(), src, local, &buf[..len]);
                    sock.set_read_timeout(Some(StdDuration::from_micros(1)))
                        .unwrap();
                }
                Err(_) => break,
            }
        }
        let n = now();
        if ep.poll_timeout().is_some_and(|t| t <= n) {
            ep.handle_timeout(n);
        }
        while let Some(t) = ep.poll_udp_transmit() {
            sock.send_to(&t.contents, t.destination).unwrap();
        }
        while let Ok(FromPage::Log(l)) = from_page.try_recv() {
            log.push(l);
        }
        std::iter::from_fn(|| ep.poll_event()).collect()
    };
    let browser_desc = |log: &mut Vec<String>| loop {
        match from_page.recv_timeout(StdDuration::from_secs(120)) {
            Ok(FromPage::Desc(d)) => break SessionDescription::from_json(&d).unwrap(),
            Ok(FromPage::Log(l)) => log.push(l),
            Err(_) => panic!("no description from the browser: {log:#?}"),
        }
    };

    let (conn, browser_sdp): (ConnId, String) = if rust_offers {
        let conn = ep.create_connection(now()).unwrap();
        let offer = loop {
            if let Poll::Ready(r) = ep.poll_local_description(now(), conn) {
                break r.unwrap();
            }
        };
        to_page.send(offer.to_json()).unwrap();
        let answer = browser_desc(&mut log);
        assert_eq!(answer.sdp_type, SdpType::Answer);
        ep.set_remote_description(now(), conn, &answer).unwrap();
        (conn, answer.sdp)
    } else {
        let offer = browser_desc(&mut log);
        assert_eq!(offer.sdp_type, SdpType::Offer);
        let conn = ep.create_connection(now()).unwrap();
        ep.set_remote_description(now(), conn, &offer).unwrap();
        let answer = loop {
            if let Poll::Ready(r) = ep.poll_local_description(now(), conn) {
                break r.unwrap();
            }
        };
        to_page.send(answer.to_json()).unwrap();
        (conn, offer.sdp)
    };
    let candidates: Vec<&str> = browser_sdp
        .lines()
        .filter(|l| l.starts_with("a=candidate"))
        .collect();
    eprintln!("browser candidates: {candidates:#?}");
    assert!(!candidates.is_empty());

    let out = stream(RUST_STREAM_SEED, SIZE);
    let want = stream(GO_STREAM_SEED, SIZE);
    let (mut sent, mut got, mut largest) = (0, Vec::with_capacity(SIZE), 0);
    let (mut open, mut blocked) = (false, false);
    let end = StdInstant::now() + StdDuration::from_secs(120);
    let ok = format!("recv-ok {SIZE} {DATA_CHANNEL_MTU}");
    while got.len() < SIZE || !log.contains(&ok) {
        assert!(
            StdInstant::now() < end,
            "timed out: sent {sent} got {} buffered {} open {open} blocked {blocked} state {:?}: {log:#?}",
            got.len(),
            ep.buffered_amount(conn),
            ep.state(conn)
        );
        assert!(
            !log.iter()
                .any(|l| l.starts_with("error") || l.starts_with("recv-bad")),
            "{log:#?}"
        );
        for e in pump(&mut ep, &mut log) {
            match e {
                Event::Open { mtu, .. } => {
                    assert_eq!(mtu, DATA_CHANNEL_MTU);
                    open = true;
                }
                Event::Data { data, .. } => {
                    largest = largest.max(data.len());
                    assert!(
                        data[..] == want[got.len()..got.len() + data.len()],
                        "browser→rust bytes differ at {}",
                        got.len()
                    );
                    got.extend_from_slice(&data);
                }
                Event::Writable(_) => blocked = false,
                Event::LocalDescription(_) => {}
                Event::Closed { reason, .. } => panic!("closed early: {reason:?} {log:#?}"),
            }
        }
        while open && !blocked && sent < SIZE {
            sent += ep.link_write(conn, &out[sent..]).unwrap();
            blocked = sent < SIZE;
        }
    }
    assert_eq!(largest, DATA_CHANNEL_MTU);

    ep.close(now(), conn);
    let end = StdInstant::now() + StdDuration::from_secs(30);
    let mut reason = None;
    while reason.is_none() || !log.iter().any(|l| l.starts_with("closed")) {
        assert!(StdInstant::now() < end, "close timed out: {log:#?}");
        for e in pump(&mut ep, &mut log) {
            if let Event::Closed { reason: r, .. } = e {
                reason = Some(r);
            }
        }
    }
    assert_eq!(reason, Some(CloseReason::Local));
    eprintln!("browser log: {log:#?}");
    assert!(log.iter().any(|l| l.starts_with("open ")), "{log:#?}");
}

/// Both roles, one after the other: snap Firefox instances must not start
/// concurrently.
// TODO: a Chrome run.
#[test]
#[ignore = "launches headless Firefox"]
fn firefox_both_roles() {
    // Firefox offers, Rust answers.
    run(false);
    // Rust offers, Firefox answers.
    run(true);
}
