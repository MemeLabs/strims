// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use js_sys::ArrayBuffer;
use strims_base::time::Duration;
use strims_rpc::Timer;
use tokio::sync::Notify;
use wasm_bindgen_test::wasm_bindgen_test;
use web_sys::MessageChannel;

use super::*;
use crate::clock::{self, WasmClock};

// --- batches ----------------------------------------------------------------

/// A message's header and bytes, as a batch holds it.
fn message(flags: u8, bytes: &[u8]) -> Vec<u8> {
    let mut m = vec![flags];
    m.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    m.extend_from_slice(bytes);
    m
}

fn batch(flags: u8, messages: &[Vec<u8>]) -> Vec<u8> {
    let mut b = vec![BATCH_DATA, flags];
    for m in messages {
        b.extend_from_slice(m);
    }
    b
}

#[wasm_bindgen_test]
fn a_batch_holds_each_record_as_a_message_ending_it() {
    let mut b = Batch::new();
    assert!(b.is_empty());
    b.piece(EgressPiece::Data(b"ab"));
    b.piece(EgressPiece::Data(b"c"));
    b.piece(EgressPiece::RecordEnd);
    b.piece(EgressPiece::Data(b"xyz"));
    b.piece(EgressPiece::RecordEnd);
    assert_eq!(b.room(), MAX_BATCH_BYTES - 6);
    assert_eq!(
        b.finish(false),
        batch(
            0,
            &[
                message(DATA_RECORD_END, b"abc"),
                message(DATA_RECORD_END, b"xyz")
            ]
        )
    );
}

#[wasm_bindgen_test]
fn a_record_a_batch_cuts_goes_on_in_the_next_batch() {
    let mut b = Batch::new();
    b.piece(EgressPiece::Data(b"ab"));
    assert_eq!(b.finish(true), batch(BATCH_ACK, &[message(0, b"ab")]));
    b.clear();
    assert!(b.is_empty());
    assert_eq!(b.room(), MAX_BATCH_BYTES);
    b.piece(EgressPiece::Data(b"cd"));
    b.piece(EgressPiece::RecordEnd);
    assert_eq!(
        b.finish(false),
        batch(0, &[message(DATA_RECORD_END, b"cd")])
    );
}

#[wasm_bindgen_test]
fn a_record_end_with_no_bytes_left_is_an_empty_message_ending_it() {
    let mut b = Batch::new();
    b.piece(EgressPiece::Data(b"ab"));
    b.finish(false);
    b.clear();
    b.piece(EgressPiece::RecordEnd);
    assert_eq!(b.finish(false), batch(0, &[message(DATA_RECORD_END, b"")]));
}

#[wasm_bindgen_test]
fn a_discontinuity_drops_the_record_in_progress_and_marks_the_next_message() {
    let mut b = Batch::new();
    b.piece(EgressPiece::Data(b"ab"));
    b.piece(EgressPiece::RecordEnd);
    b.piece(EgressPiece::Data(b"cd"));
    b.piece(EgressPiece::Discontinuity);
    assert_eq!(b.room(), MAX_BATCH_BYTES - 2);
    b.piece(EgressPiece::Data(b"ef"));
    b.piece(EgressPiece::RecordEnd);
    b.piece(EgressPiece::Data(b"g"));
    b.piece(EgressPiece::RecordEnd);
    assert_eq!(
        b.finish(false),
        batch(
            0,
            &[
                message(DATA_RECORD_END, b"ab"),
                message(DATA_RECORD_END | DATA_DISCONTINUITY, b"ef"),
                message(DATA_RECORD_END, b"g"),
            ]
        )
    );
}

#[wasm_bindgen_test]
fn a_discontinuity_at_a_batch_s_end_marks_the_next_batch() {
    let mut b = Batch::new();
    b.piece(EgressPiece::Data(b"ab"));
    b.piece(EgressPiece::Discontinuity);
    assert!(b.is_empty());
    b.piece(EgressPiece::Data(b"cd"));
    assert_eq!(
        b.finish(false),
        batch(0, &[message(DATA_DISCONTINUITY, b"cd")])
    );
}

// --- the window -------------------------------------------------------------

#[wasm_bindgen_test]
fn the_window_closes_at_its_batches_and_asks_for_acks_every_quarter() {
    let mut c = Credit::default();
    let mut asked = Vec::new();
    for i in 1..=WINDOW_BATCHES {
        assert!(c.is_open(), "open before batch {i}");
        if c.post(1) {
            asked.push(i);
        }
    }
    assert!(!c.is_open());
    let q = WINDOW_BATCHES / 4;
    assert_eq!(asked, vec![q, 2 * q, 3 * q, 4 * q]);
    c.ack(q as u64);
    assert!(c.is_open());
    assert_eq!(c.unacked(), (WINDOW_BATCHES - q, WINDOW_BATCHES - q));
}

#[wasm_bindgen_test]
fn the_window_closes_at_its_bytes_and_the_batch_that_closes_it_asks() {
    let mut c = Credit::default();
    let small = WINDOW_BYTES / 8;
    assert!(!c.post(small));
    assert!(c.post(small), "a quarter of the bytes");
    assert!(!c.post(small));
    assert!(c.is_open());
    assert!(c.post(WINDOW_BYTES), "the batch that closes the window");
    assert!(!c.is_open());
    c.ack(3);
    assert!(!c.is_open(), "the big batch is still unacked");
    c.ack(4);
    assert!(c.is_open());
    assert_eq!(c.unacked(), (0, 0));
}

#[wasm_bindgen_test]
fn acks_past_the_batches_posted_count_as_all_and_old_acks_change_nothing() {
    let mut c = Credit::default();
    c.post(10);
    c.post(20);
    c.ack(1);
    assert_eq!(c.unacked(), (20, 1));
    c.ack(0);
    assert_eq!(c.unacked(), (20, 1));
    c.ack(1_000_000);
    assert_eq!(c.unacked(), (0, 0));
    c.post(5);
    assert_eq!(
        c.unacked(),
        (5, 1),
        "an ack counts batches posted before it"
    );
    c.ack(3);
    assert_eq!(c.unacked(), (0, 0));
    assert_eq!(c.posted(), 3);
}

// --- pumping ----------------------------------------------------------------

enum Step {
    Data(Vec<u8>),
    End,
    Gap,
    /// The read waits until the gate is opened.
    Wait,
    Fail(VideoError),
}

/// A stream that yields its steps in order, and waits forever after the
/// last.
struct Script {
    steps: Rc<RefCell<VecDeque<Step>>>,
    gate: Rc<Notify>,
    buf: Vec<u8>,
}

impl Script {
    fn new(steps: Vec<Step>) -> (Script, Rc<RefCell<VecDeque<Step>>>, Rc<Notify>) {
        let steps = Rc::new(RefCell::new(VecDeque::from(steps)));
        let gate = Rc::new(Notify::new());
        let s = Script {
            steps: steps.clone(),
            gate: gate.clone(),
            buf: Vec::new(),
        };
        (s, steps, gate)
    }
}

impl Pieces for Script {
    async fn next(&mut self, max: usize) -> Result<EgressPiece<'_>, VideoError> {
        loop {
            let step = self.steps.borrow_mut().pop_front();
            match step {
                Some(Step::Data(mut d)) => {
                    if d.len() > max {
                        let rest = d.split_off(max);
                        self.steps.borrow_mut().push_front(Step::Data(rest));
                    }
                    self.buf = d;
                    return Ok(EgressPiece::Data(&self.buf));
                }
                Some(Step::End) => return Ok(EgressPiece::RecordEnd),
                Some(Step::Gap) => return Ok(EgressPiece::Discontinuity),
                Some(Step::Fail(e)) => return Err(e),
                Some(Step::Wait) => self.gate.notified().await,
                None => std::future::pending::<()>().await,
            }
        }
    }
}

/// What reached the worker's end of a feed's port.
struct Worker {
    port: MessagePort,
    batches: Rc<RefCell<Vec<Vec<u8>>>>,
    arrived: Rc<Notify>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl Worker {
    fn new(port: MessagePort) -> Worker {
        let batches = Rc::new(RefCell::new(Vec::new()));
        let arrived = Rc::new(Notify::new());
        let (b, a) = (batches.clone(), arrived.clone());
        let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |e: MessageEvent| {
            let buffer: ArrayBuffer = e.data().dyn_into().unwrap();
            b.borrow_mut().push(Uint8Array::new(&buffer).to_vec());
            a.notify_one();
        });
        port.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
        Worker {
            port,
            batches,
            arrived,
            _on_message: on_message,
        }
    }

    /// Waits until `n` batches have arrived.
    async fn until(&self, n: usize) {
        let deadline = WasmClock.sleep_until(clock::now() + Duration::from_secs(10));
        let mut deadline = pin!(deadline);
        while self.batches.borrow().len() < n {
            tokio::select! {
                biased;
                () = self.arrived.notified() => {}
                () = &mut deadline => panic!("{} of {n} batches arrived", self.batches.borrow().len()),
            }
        }
    }

    fn ack(&self, taken: usize) {
        self.port
            .post_message(&JsValue::from_f64(taken as f64))
            .unwrap();
    }
}

async fn sleep(ms: i64) {
    WasmClock
        .sleep_until(clock::now() + Duration::from_millis(ms))
        .await;
}

/// Runs `pump` on `script` in a task: where its result lands.
fn spawn_pump(mut script: Script, port: MessagePort) -> Rc<RefCell<Option<Stopped>>> {
    let out = Rc::new(RefCell::new(None));
    let o = out.clone();
    drop(runtime::spawn(async move {
        let port = FeedPort::new(port);
        let stopped = pump(&mut script, &port).await;
        *o.borrow_mut() = Some(stopped);
    }));
    out
}

#[wasm_bindgen_test]
async fn a_feed_posts_what_the_stream_has_ready_as_one_batch_before_it_waits() {
    let channel = MessageChannel::new().unwrap();
    let worker = Worker::new(channel.port2());
    let (script, _, gate) = Script::new(vec![
        Step::Data(b"ab".to_vec()),
        Step::End,
        Step::Data(b"c".to_vec()),
        Step::End,
        Step::Wait,
        Step::Data(b"d".to_vec()),
        Step::Gap,
        Step::Data(b"e".to_vec()),
        Step::End,
        Step::Fail(VideoError::StreamClosed),
    ]);
    let stopped = spawn_pump(script, channel.port1());
    worker.until(1).await;
    assert_eq!(
        worker.batches.borrow()[0],
        batch(
            0,
            &[
                message(DATA_RECORD_END, b"ab"),
                message(DATA_RECORD_END, b"c")
            ]
        )
    );
    gate.notify_one();
    worker.until(2).await;
    assert_eq!(
        worker.batches.borrow()[1],
        batch(0, &[message(DATA_RECORD_END | DATA_DISCONTINUITY, b"e")])
    );
    sleep(20).await;
    assert_eq!(
        *stopped.borrow(),
        Some(Stopped::Stream(VideoError::StreamClosed))
    );
}

/// A data batch's messages: each one's flags and length.
fn messages(batch: &[u8]) -> Vec<(u8, usize)> {
    assert_eq!(batch[0], BATCH_DATA);
    let mut at = BATCH_HEADER;
    let mut out = Vec::new();
    while at < batch.len() {
        let len = u32::from_le_bytes(batch[at + 1..at + 5].try_into().unwrap()) as usize;
        out.push((batch[at], len));
        at += MESSAGE_HEADER + len;
    }
    assert_eq!(at, batch.len(), "messages fill the batch");
    out
}

#[wasm_bindgen_test]
async fn a_feed_reads_no_further_than_its_window_until_the_worker_acks() {
    let channel = MessageChannel::new().unwrap();
    let worker = Worker::new(channel.port2());
    let record = vec![7u8; MAX_BATCH_BYTES / 2];
    let full = WINDOW_BYTES / MAX_BATCH_BYTES;
    let records = 2 * full + 8;
    let mut steps = Vec::new();
    for _ in 0..records {
        steps.push(Step::Data(record.clone()));
        steps.push(Step::End);
    }
    steps.push(Step::Wait);
    let (script, left, _gate) = Script::new(steps);
    let _stopped = spawn_pump(script, channel.port1());

    // Two records fill a batch, and the window closes on its bytes at
    // the `full`th, which asks for an ack, as every quarter's last does.
    worker.until(full).await;
    sleep(100).await;
    assert_eq!(worker.batches.borrow().len(), full, "the window is full");
    // Read: the window's records, all but the last one's end.
    assert_eq!(left.borrow().len(), 1 + 2 * (records - 2 * full) + 1);
    let asks: Vec<usize> = worker
        .batches
        .borrow()
        .iter()
        .enumerate()
        .filter(|(_, b)| b[1] & BATCH_ACK != 0)
        .map(|(i, _)| i + 1)
        .collect();
    assert_eq!(asks, vec![full / 4, full / 2, 3 * full / 4, full]);

    worker.ack(full);
    while left.borrow().len() > 1 {
        sleep(10).await;
    }
    sleep(50).await;
    let batches = worker.batches.borrow();
    let all: Vec<(u8, usize)> = batches.iter().flat_map(|b| messages(b)).collect();
    let bytes: usize = all.iter().map(|(_, len)| len).sum();
    let ends = all.iter().filter(|(f, _)| f & DATA_RECORD_END != 0).count();
    assert_eq!((bytes, ends), (records * record.len(), records));
    assert!(all.iter().all(|(f, _)| f & DATA_DISCONTINUITY == 0));
}

#[wasm_bindgen_test]
async fn a_feed_s_last_batch_goes_before_it_stops_at_a_failed_read() {
    let channel = MessageChannel::new().unwrap();
    let worker = Worker::new(channel.port2());
    let (script, _, _) = Script::new(vec![
        Step::Data(b"ab".to_vec()),
        Step::End,
        Step::Data(b"c".to_vec()),
        Step::Fail(VideoError::StreamRead("gone".to_owned())),
    ]);
    let stopped = spawn_pump(script, channel.port1());
    worker.until(1).await;
    assert_eq!(
        worker.batches.borrow()[0],
        batch(0, &[message(DATA_RECORD_END, b"ab"), message(0, b"c")])
    );
    sleep(20).await;
    assert_eq!(
        *stopped.borrow(),
        Some(Stopped::Stream(VideoError::StreamRead("gone".to_owned())))
    );
}

#[wasm_bindgen_test]
async fn the_end_batch_says_the_stream_ended() {
    let channel = MessageChannel::new().unwrap();
    let worker = Worker::new(channel.port2());
    let port = FeedPort::new(channel.port1());
    port.end();
    worker.until(1).await;
    assert_eq!(worker.batches.borrow()[0], vec![BATCH_END]);
}
