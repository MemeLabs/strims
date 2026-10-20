// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `ppspp-capture`: reads ppspp capture dumps.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::process::ExitCode;

use strims_ppspp_capture::{Clock, Dir, Error, Reader, Record, export, tap_kind};

const USAGE: &str = "\
usage: ppspp-capture <command> [file...]

Reads ppspp capture dumps: each file, or standard input for none or `-`.

commands:
  export   one JSON object per line: a row per frame, per ppspp message
           and per scheduler decision that annotates no message (the
           crate's export module documents the fields)
  info     each file's header and swarms, and what it holds
  help     print this";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let mut files: Vec<String> = args.collect();
    if files.is_empty() {
        files.push("-".into());
    }
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    let mut ok = true;
    for path in &files {
        let input: Box<dyn Read> = if path == "-" {
            Box::new(io::stdin().lock())
        } else {
            match File::open(path) {
                Ok(f) => Box::new(BufReader::new(f)),
                Err(e) => {
                    eprintln!("{path}: {e}");
                    ok = false;
                    continue;
                }
            }
        };
        let mut reader = match Reader::new(input) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("{path}: {e}");
                ok = false;
                continue;
            }
        };
        let mut on_error = |e: &Error| {
            eprintln!("{path}: {e}");
            ok = false;
        };
        let res = match cmd.as_str() {
            "export" => export(&mut reader, &mut out, &mut on_error).map(|_| ()),
            "info" => info(path, &mut reader, &mut out, &mut on_error),
            "help" | "--help" | "-h" => {
                println!("{USAGE}");
                return ExitCode::SUCCESS;
            }
            _ => {
                eprintln!("unknown command {cmd}\n\n{USAGE}");
                return ExitCode::FAILURE;
            }
        };
        match res.and_then(|()| out.flush()) {
            Ok(()) => {}
            // the reader went away, e.g. `| head`
            Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("write: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
    if ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// What a file holds, in a few lines.
fn info<R: Read>(
    path: &str,
    r: &mut Reader<R>,
    out: &mut impl Write,
    on_error: &mut dyn FnMut(&Error),
) -> io::Result<()> {
    let h = r.header().clone();
    let clock = match h.clock {
        Clock::Sim => "sim".to_string(),
        Clock::Node => "node".to_string(),
        Clock::Other(v) => format!("unknown ({v})"),
    };
    writeln!(
        out,
        "{path}: version {}, clock {clock}, node {} {:?}, file {}, origin {} ns",
        h.version,
        hex(&h.node),
        h.label,
        h.file,
        h.origin.unix_nanos()
    )?;
    let (mut frames, mut bytes) = ([0u64; 2], [0u64; 2]);
    let (mut records, mut taps, mut lost, mut errors, mut peers) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let mut last = h.origin;
    let mut by_kind = std::collections::BTreeMap::<&str, u64>::new();
    while let Some(e) = r.next_record() {
        let e = match e {
            Ok(e) => e,
            Err(e) => {
                errors += 1;
                on_error(&e);
                continue;
            }
        };
        records += 1;
        last = e.time;
        match e.record {
            Record::Peer { .. } => peers += 1,
            Record::Swarm { swarm, id, params } => {
                writeln!(
                    out,
                    "  swarm {swarm} {}: chunks of {} bytes, {} streams, window {} chunks, \
                     hashes of {}, signatures of {}, {} per signature, push {}, \
                     controller {}, seed allocation {}, seed qos {}, scheduling {}",
                    hex(id),
                    params.chunk_size,
                    params.stream_count,
                    params.live_window,
                    params.integrity_hash_size,
                    params.integrity_signature_size,
                    params.chunks_per_signature,
                    name(params.push_mode, &["off", "eager have", "hybrid"]),
                    name(params.request_controller, &["ledbat", "bbr-like"]),
                    name(params.seed_allocation, &["admission", "adaptive"]),
                    name(params.seed_qos, &["equal", "by allocation"]),
                    name(params.scheduling_method, &["seed", "peer"]),
                )?;
            }
            Record::Frame { dir, len, .. } => {
                let i = usize::from(dir == Dir::Out);
                if let (Some(f), Some(b)) = (frames.get_mut(i), bytes.get_mut(i)) {
                    *f += 1;
                    *b += len;
                }
            }
            Record::Tap(t) => {
                taps += 1;
                *by_kind
                    .entry(tap_kind(t.kind).map_or("unknown", |k| k.name))
                    .or_default() += 1;
            }
            Record::TapLost { count } => lost += count,
            _ => {}
        }
    }
    let [fi, fo] = frames;
    let [bi, bo] = bytes;
    writeln!(
        out,
        "  {records} records over {:.3} s, {peers} peers; frames in {fi} ({bi} bytes), out {fo} ({bo} bytes); {taps} decisions, {lost} lost; {errors} errors",
        (last - h.origin).as_secs_f64()
    )?;
    if !by_kind.is_empty() {
        let kinds: Vec<String> = by_kind.iter().map(|(k, n)| format!("{k} {n}")).collect();
        writeln!(out, "  decisions: {}", kinds.join(", "))?;
    }
    Ok(())
}

/// The name of a [`SwarmParams`](strims_ppspp_capture::SwarmParams) value
/// from 1.
fn name(v: u64, names: &[&str]) -> String {
    v.checked_sub(1)
        .and_then(|i| names.get(i as usize))
        .map_or_else(|| format!("unknown ({v})"), |n| (*n).to_string())
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}
