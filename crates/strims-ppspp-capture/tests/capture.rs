// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Captures of two runners wired back to back with instant delivery.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::RefCell;
use std::io::{self, Write};
use std::rc::Rc;

use strims_base::rng::det_rng;
use strims_base::time::{Duration, Time};
use strims_binmap::Bin;
use strims_ppspp::codec::{FrameReader, Message, Reader as MessageReader, ReaderConfig};
use strims_ppspp::{
    PeerKey, PushMode, Runner, Swarm, SwarmId, SwarmKey, SwarmOptions, Writer, WriterOptions,
};
use strims_ppspp_capture::{Capture, Clock, Dir, Options, Reader, Record, SwarmParams, tap_kind};

const MTU: usize = 16 * 1024 - 4;

fn t(ms: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + ms)
}

/// Files kept in memory, shared with the test.
#[derive(Clone, Default)]
struct Files(Rc<RefCell<Vec<Vec<u8>>>>);

struct File {
    files: Files,
    index: usize,
    /// Fail writes past this many bytes.
    limit: usize,
}

impl Write for File {
    fn write(&mut self, b: &[u8]) -> io::Result<usize> {
        let mut files = self.files.0.borrow_mut();
        let f = &mut files[self.index];
        if f.len() + b.len() > self.limit {
            return Err(io::Error::other("disk full"));
        }
        f.extend_from_slice(b);
        Ok(b.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Files {
    fn sink(&self, limit: usize) -> impl FnMut(u64) -> io::Result<File> + use<> {
        let files = self.clone();
        move |i| {
            let mut v = files.0.borrow_mut();
            assert_eq!(v.len() as u64, i);
            v.push(Vec::new());
            Ok(File {
                files: files.clone(),
                index: i as usize,
                limit,
            })
        }
    }

    fn get(&self) -> Vec<Vec<u8>> {
        self.0.borrow().clone()
    }
}

type Cap = Capture<Box<dyn FnMut(u64) -> io::Result<File>>>;

struct Node {
    runner: Runner,
    swarm: SwarmKey,
    peer: PeerKey,
    cap: Option<Cap>,
}

struct Pair {
    nodes: [Node; 2],
    writer: Writer,
    now: Time,
}

fn options() -> SwarmOptions {
    SwarmOptions {
        live_window: 1 << 10,
        push_mode: PushMode::Hybrid,
        ..Default::default()
    }
}

impl Pair {
    fn new() -> Pair {
        let key = strims_crypto::key_from_seed(&[4; 32]);
        let (writer, swarm) = Writer::new(
            WriterOptions {
                swarm_options: options(),
                key: key.clone(),
            },
            t(0),
        )
        .unwrap();
        let mut seed = Runner::new(det_rng(1));
        let ss = seed.add_swarm(swarm);
        let sp = seed.add_peer(b"leech", t(0));
        seed.run_swarm(sp, ss, 1, 2, t(0)).unwrap();
        seed.pin_seed_allocation(ss, Some(vec![vec![sp]])).unwrap();

        let swarm = Swarm::new(SwarmId::new(&key.public), options()).unwrap();
        let mut leech = Runner::new(det_rng(2));
        let ls = leech.add_swarm(swarm);
        let lp = leech.add_peer(b"seed", t(0));
        leech.run_swarm(lp, ls, 2, 1, t(0)).unwrap();
        Pair {
            nodes: [
                Node {
                    runner: seed,
                    swarm: ss,
                    peer: sp,
                    cap: None,
                },
                Node {
                    runner: leech,
                    swarm: ls,
                    peer: lp,
                    cap: None,
                },
            ],
            writer,
            now: t(0),
        }
    }

    fn capture(&mut self, i: usize, sink: Box<dyn FnMut(u64) -> io::Result<File>>, max: u64) {
        let n = &mut self.nodes[i];
        let name = if i == 0 { "seed" } else { "leech" };
        n.cap = Some(
            Capture::new(
                sink,
                Options {
                    clock: Clock::Sim,
                    node: name.as_bytes(),
                    label: name,
                    max_file_bytes: max,
                    tap: true,
                },
                self.now,
                &mut n.runner,
            )
            .unwrap(),
        );
    }

    fn pump(&mut self) {
        let mut buf = vec![0u8; MTU];
        let now = self.now;
        loop {
            let mut moved = false;
            for from in [0, 1] {
                let [a, b] = if from == 0 {
                    self.nodes.each_mut()
                } else {
                    let [x, y] = self.nodes.each_mut();
                    [y, x]
                };
                while a.runner.wants_transmit(a.peer) {
                    let n = match a.cap.as_mut() {
                        Some(c) => c.poll_transmit(&mut a.runner, a.peer, now, &mut buf),
                        None => a.runner.poll_transmit(a.peer, now, &mut buf),
                    };
                    if n == 0 {
                        break;
                    }
                    moved = true;
                    match b.cap.as_mut() {
                        Some(c) => c.handle_frame(&mut b.runner, b.peer, now, &buf[..n]),
                        None => b.runner.handle_frame(b.peer, now, &buf[..n]),
                    }
                    .unwrap();
                }
            }
            if !moved {
                return;
            }
        }
    }

    fn tick(&mut self, ms: i64) {
        self.now += Duration::from_millis(ms);
        for n in &mut self.nodes {
            n.runner.handle_timeout(self.now);
            if let Some(c) = n.cap.as_mut() {
                c.drain(self.now, &mut n.runner);
            }
        }
        self.pump();
    }

    fn write(&mut self, bytes: usize) {
        let data = vec![7u8; bytes];
        let n = &mut self.nodes[0];
        n.runner
            .write(n.swarm, &mut self.writer, self.now, &data)
            .unwrap();
        if let Some(c) = n.cap.as_mut() {
            c.drain(self.now, &mut n.runner);
        }
    }

    /// Pulled at first, pushed once the leech subscribes.
    fn run(&mut self) {
        self.tick(100);
        self.write(128 * 1024);
        for _ in 0..10 {
            self.tick(50);
        }
        let n = &mut self.nodes[1];
        n.runner.request_stream(n.peer, n.swarm, 0, Bin(0)).unwrap();
        for _ in 0..10 {
            self.write(64 * 1024);
            for _ in 0..5 {
                self.tick(50);
            }
        }
    }
}

/// A file's records, as (time, debug string), and its counts.
#[derive(Default, Debug)]
struct Summary {
    frames_in: u64,
    bytes_in: u64,
    frames_out: u64,
    bytes_out: u64,
    /// Chunks in DATA messages out.
    data_out: u64,
    /// Chunks in DataSent tap records, pushed and answered.
    sent_push: u64,
    sent_answer: u64,
    hits: u64,
    swarm: Option<SwarmParams>,
    /// Records before the first frame, by kind.
    lead: Vec<&'static str>,
}

fn summarize(bytes: &[u8]) -> Summary {
    let mut r = Reader::new(bytes).unwrap();
    let mut s = Summary::default();
    let mut cfg = None;
    let mut framed = false;
    while let Some(e) = r.next_record() {
        let e = e.unwrap();
        match e.record {
            Record::Swarm { params, .. } => {
                s.swarm = Some(params);
                cfg = Some(ReaderConfig {
                    chunk_size: params.chunk_size as usize,
                    integrity_hash_size: params.integrity_hash_size as usize,
                    integrity_signature_size: params.integrity_signature_size as usize,
                });
            }
            Record::Frame { dir, len, body, .. } => {
                framed = true;
                assert_eq!(len, body.len() as u64);
                match dir {
                    Dir::In => {
                        s.frames_in += 1;
                        s.bytes_in += len;
                    }
                    Dir::Out => {
                        s.frames_out += 1;
                        s.bytes_out += len;
                        for c in FrameReader::new(body) {
                            let (_, payload) = c.unwrap();
                            for m in MessageReader::new(cfg.unwrap(), payload) {
                                if let Message::Data(d) = m.unwrap() {
                                    s.data_out += d.bin.base_length();
                                }
                            }
                        }
                    }
                }
            }
            Record::Tap(t) => {
                let name = tap_kind(t.kind).unwrap().name;
                let n = Bin(t.field(0) as u64).base_length();
                match name {
                    "DataSent" if t.field(1) == 1 => s.sent_push += n,
                    "DataSent" => s.sent_answer += n,
                    "PushHit" => s.hits += 1,
                    _ => {}
                }
            }
            _ => {}
        }
        if !framed {
            s.lead.push(match e.record {
                Record::Peer { .. } => "peer",
                Record::Swarm { .. } => "swarm",
                Record::ChannelOpen { .. } => "channel",
                Record::Tap(_) => "tap",
                _ => "other",
            });
        }
    }
    s
}

#[test]
fn a_capture_holds_frames_and_decisions() {
    let files = [Files::default(), Files::default()];
    let mut p = Pair::new();
    p.capture(0, Box::new(files[0].sink(usize::MAX)), 0);
    p.capture(1, Box::new(files[1].sink(usize::MAX)), 0);
    p.run();

    let seed = summarize(&files[0].get()[0]);
    let leech = summarize(&files[1].get()[0]);
    let stats = p.nodes[0].cap.as_ref().unwrap().stats();
    // The snapshot, then decisions made before the first frame.
    assert_eq!(seed.lead[..3], ["peer", "swarm", "channel"]);
    assert!(
        seed.lead[3..].iter().all(|&k| k == "tap"),
        "{:?}",
        seed.lead
    );
    let params = seed.swarm.unwrap();
    assert_eq!((params.chunk_size, params.stream_count), (1024, 1));
    assert_eq!((params.push_mode, params.scheduling_method), (3, 1));
    assert_eq!(leech.swarm.unwrap().scheduling_method, 2);

    assert_eq!(
        (seed.frames_out, seed.bytes_out),
        (stats.frames_out, stats.bytes_out)
    );
    assert_eq!(
        (seed.frames_out, seed.bytes_out),
        (leech.frames_in, leech.bytes_in)
    );
    assert_eq!(
        (leech.frames_out, leech.bytes_out),
        (seed.frames_in, seed.bytes_in)
    );
    // every chunk sent is a push or an answer
    assert!(seed.sent_push > 0 && seed.sent_answer > 0, "{seed:?}");
    assert_eq!(seed.sent_push + seed.sent_answer, seed.data_out);
    let s = p.nodes[0].runner.swarm_stats(p.nodes[0].swarm).unwrap();
    assert_eq!(seed.sent_push, s.pushed_chunks);
    assert!(leech.hits > 0);
}

#[test]
fn a_capture_without_the_tap_keeps_the_lifecycle() {
    let files = Files::default();
    let mut p = Pair::new();
    let n = &mut p.nodes[1];
    n.cap = Some(
        Capture::new(
            Box::new(files.sink(usize::MAX)) as Box<dyn FnMut(u64) -> io::Result<File>>,
            Options {
                clock: Clock::Sim,
                node: b"leech",
                label: "",
                max_file_bytes: 0,
                tap: false,
            },
            p.now,
            &mut n.runner,
        )
        .unwrap(),
    );
    p.run();
    let n = &mut p.nodes[1];
    n.runner.stop_swarm(n.peer, n.swarm);
    n.cap.as_mut().unwrap().drain(p.now, &mut n.runner);
    let bytes = files.get().remove(0);
    let mut r = Reader::new(&bytes[..]).unwrap();
    let mut closed = false;
    while let Some(e) = r.next_record() {
        match e.unwrap().record {
            Record::Tap(_) => panic!("a tap record"),
            Record::ChannelClose { peer: 1, swarm: 1 } => closed = true,
            _ => {}
        }
    }
    assert!(closed);
}

#[test]
fn rotated_files_each_decode_on_their_own() {
    let whole = Files::default();
    let rotated = Files::default();
    let mut a = Pair::new();
    a.capture(0, Box::new(whole.sink(usize::MAX)), 0);
    a.run();
    let mut b = Pair::new();
    b.capture(0, Box::new(rotated.sink(usize::MAX)), 64 * 1024);
    b.run();

    let whole = summarize(&whole.get()[0]);
    let rotated = rotated.get();
    assert!(rotated.len() > 3, "{}", rotated.len());
    let (mut frames, mut bytes) = (0, 0);
    for (i, f) in rotated.iter().enumerate() {
        let r = Reader::new(&f[..]).unwrap();
        assert_eq!(r.header().file, i as u64);
        // a file's size passes the limit by at most what one drain wrote
        assert!(f.len() < 64 * 1024 + 2 * MTU, "{}", f.len());
        let s = summarize(f);
        assert_eq!(s.lead[..3], ["peer", "swarm", "channel"]);
        frames += s.frames_out;
        bytes += s.bytes_out;
    }
    assert_eq!((frames, bytes), (whole.frames_out, whole.bytes_out));
    let stats = b.nodes[0].cap.as_ref().unwrap().stats();
    assert_eq!(stats.files, rotated.len() as u64);
    assert_eq!(
        stats.written,
        rotated.iter().map(|f| f.len() as u64).sum::<u64>()
    );
}

#[test]
fn a_failed_write_stops_the_capture() {
    let files = Files::default();
    let mut p = Pair::new();
    p.capture(0, Box::new(files.sink(16 * 1024)), 0);
    p.run();
    let c = p.nodes[0].cap.as_ref().unwrap();
    assert!(c.error().is_some());
    let mut events = 0;
    p.nodes[0].runner.drain_tap(|_| events += 1);
    assert_eq!(events, 0, "the tap is off");
    // what was written still reads
    let f = &files.get()[0];
    let mut r = Reader::new(&f[..]).unwrap();
    while let Some(e) = r.next_record() {
        if e.is_err() {
            break;
        }
    }
}
