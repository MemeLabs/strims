// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `ppspp-capture`: reads ppspp capture dumps.

use std::fs::File;
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use strims_ppspp_capture::{
    Clock, Dir, Error, Reader, Record, export, sample_kind, swarm::swarm, tap_kind,
};

const USAGE: &str = "\
usage: ppspp-capture <command> [file...]

Reads ppspp capture dumps: each file, or standard input for none or `-`.

commands:
  export   one JSON object per line: a row per frame, per ppspp message,
           per scheduler decision that annotates no message and per
           sample (the crate's export module documents the fields)
  swarm    the export rows of several nodes' dumps (files, or directories
           of .ppcap files) in time order, with each DATA in tied to its
           DATA out and each refusal a child hears to its parent's
           decision (the crate's swarm module documents them)
  info     each file's header and swarms, and what it holds
  help     print this";

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(cmd) = args.next() else {
        eprintln!("{USAGE}");
        return ExitCode::FAILURE;
    };
    let mut files: Vec<String> = args.collect();
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());
    if cmd == "swarm" {
        return swarm_command(&files, &mut out);
    }
    if files.is_empty() {
        files.push("-".into());
    }
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

/// `swarm`: the dumps named, and those in the directories named.
fn swarm_command(args: &[String], out: &mut impl Write) -> ExitCode {
    let mut paths = Vec::new();
    for a in args {
        let p = PathBuf::from(a);
        if p.is_dir() {
            let entries = match std::fs::read_dir(&p) {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("{a}: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let mut found: Vec<PathBuf> = entries
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|x| x == "ppcap"))
                .collect();
            found.sort();
            paths.extend(found);
        } else {
            paths.push(p);
        }
    }
    if paths.is_empty() {
        eprintln!("swarm: no dumps\n\n{USAGE}");
        return ExitCode::FAILURE;
    }
    let mut ok = true;
    let res = swarm(&paths, out, &mut |e| {
        eprintln!("{e}");
        ok = false;
    });
    match res.and_then(|s| out.flush().map(|()| s)) {
        Ok(s) => {
            eprintln!(
                "{} nodes, {} rows, {} frames; {} DATA and {} refusals tied; {} errors",
                s.nodes, s.rows, s.frames, s.data_joined, s.decisions_joined, s.errors
            );
        }
        Err(e) if e.kind() == io::ErrorKind::BrokenPipe => return ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("swarm: {e}");
            return ExitCode::FAILURE;
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
    let mut by_source = std::collections::BTreeMap::<&str, u64>::new();
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
            Record::Sample(s) => {
                *by_source
                    .entry(sample_kind(s.source).map_or("unknown", |k| k.name))
                    .or_default() += 1;
            }
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
    if !by_source.is_empty() {
        let sources: Vec<String> = by_source.iter().map(|(k, n)| format!("{k} {n}")).collect();
        writeln!(out, "  samples: {}", sources.join(", "))?;
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
