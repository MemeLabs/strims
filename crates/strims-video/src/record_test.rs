// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Test code: a panic or out-of-range index is a test failure.
#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use super::*;

#[test]
fn packed_sink_outputs_frames_as_they_become_final() {
    let mut sink = PackedSink::with_size(16).unwrap();
    sink.write(b"abcdef");
    assert!(sink.output().is_empty());
    sink.end_record().unwrap();
    // The frame up to the end: header 0x8000_000a, then the record.
    assert_eq!(sink.output(), b"\x80\x00\x00\x0aabcdef");
    // A second record can't end in the frame until it fills it.
    sink.write(b"gh");
    assert!(sink.end_blocked_after(0));
    assert!(sink.end_blocked_after(3));
    assert!(!sink.end_blocked_after(4));
    assert_eq!(sink.unblocking_len(1), 4);
    assert_eq!(sink.end_record(), Err(FrameHasEnd));
    sink.write(b"ijkl");
    assert!(!sink.end_blocked_after(0));
    sink.end_record().unwrap();
    // The record filled the frame: its end is in the next frame's header.
    assert_eq!(sink.offset(), 16 + 4);
    assert_eq!(
        sink.output(),
        b"\x80\x00\x00\x0aabcdefghijkl\x80\x00\x00\x04".as_slice()
    );
}

#[test]
fn restore_reuses_the_buffer_and_keeps_newer_output() {
    let mut sink = PackedSink::new();
    sink.write(&[7; 100]);
    sink.end_record().unwrap();
    let out = sink.take_output();
    assert_eq!(out.len(), 104);
    let capacity = out.capacity();
    sink.restore(out);
    assert!(sink.output().is_empty());
    sink.write(&[8; DEFAULT_SIZE]);
    assert_eq!(sink.output().len(), DEFAULT_SIZE - 104);
    assert!(sink.take_output().capacity() >= capacity);

    sink.write(&[9; DEFAULT_SIZE]);
    let out = sink.take_output();
    sink.write(&[10; DEFAULT_SIZE]);
    let newer = sink.output().to_vec();
    assert!(!newer.is_empty());
    sink.restore(out);
    assert_eq!(sink.output(), newer);
}

#[test]
fn vec_sink_collects_whole_records() {
    let mut sink = VecSink::new();
    sink.write(b"ab");
    sink.write(b"c");
    assert_eq!(sink.current(), b"abc");
    assert!(!sink.end_blocked_after(0));
    assert_eq!(sink.unblocking_len(5), 5);
    sink.end_record().unwrap();
    sink.end_record().unwrap();
    sink.write(b"d");
    assert_eq!(sink.records, [b"abc".to_vec(), Vec::new()]);
    assert_eq!(sink.current(), b"d");
}

#[test]
fn a_borrowed_sink_is_a_sink() {
    let mut sink = VecSink::new();
    {
        let mut borrowed = &mut sink;
        RecordSink::write(&mut borrowed, b"x");
        RecordSink::end_record(&mut borrowed).unwrap();
    }
    assert_eq!(sink.records, [b"x".to_vec()]);
}
