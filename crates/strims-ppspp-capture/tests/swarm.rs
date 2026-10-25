// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The swarm view of two nodes' dumps: rows in time order, a push tied to
//! its arrival and a refusal to its parent's decision.

#![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

use std::path::{Path, PathBuf};

use serde_json::Value;
use strims_base::time::Time;
use strims_binmap::Bin;
use strims_ppspp::codec::{ChannelWriter, Data, Message, MessageWriter};
use strims_ppspp_capture::{
    Clock, Dir, Header, Record, SwarmParams, TapRecord, Writer, swarm::swarm, tap_kind,
};

fn t(ms: i64) -> Time {
    Time::from_unix_millis(1_700_000_000_000 + ms)
}

fn code(name: &str) -> u32 {
    (1..64)
        .find(|&c| tap_kind(c).is_some_and(|k| k.name == name))
        .unwrap()
}

fn data_frame(channel: u64) -> Vec<u8> {
    let mut buf = vec![0u8; 256];
    let mut cw = ChannelWriter::new(&mut buf, channel);
    cw.write(&Message::Data(Data {
        bin: Bin(2),
        timestamp: t(10),
        data: &[1, 2, 3, 4],
    }))
    .unwrap();
    let n = cw.finish();
    buf.truncate(n);
    buf
}

/// A dump by `node` whose one peer is `other`, on channel `ours` (frames
/// from the peer) and `theirs` (frames to it).
fn dump(dir: &Path, node: u8, other: u8, ours: u64, theirs: u64, recs: &[(Time, Record<'_>)]) {
    let params = SwarmParams {
        chunk_size: 4,
        integrity_hash_size: 32,
        integrity_signature_size: 64,
        stream_count: 2,
        live_window: 64,
        ..SwarmParams::default()
    };
    let mut w = Writer::new(
        Vec::new(),
        &Header {
            clock: Clock::Sim,
            origin: t(0),
            file: 0,
            node: &[node],
            label: "",
        },
    )
    .unwrap();
    let head = [
        Record::Peer {
            peer: 1,
            name: &[other],
        },
        Record::Swarm {
            swarm: 1,
            id: &[7; 32],
            params,
        },
        Record::ChannelOpen {
            peer: 1,
            swarm: 1,
            channel: ours,
            peer_channel: theirs,
        },
    ];
    for r in &head {
        w.write(t(0), r).unwrap();
    }
    for (at, r) in recs {
        w.write(*at, r).unwrap();
    }
    std::fs::write(dir.join(format!("{node:02x}-0.ppcap")), w.into_inner()).unwrap();
}

#[test]
fn a_swarm_view_ties_rows_across_nodes() {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "a_swarm_view_ties_rows_across_nodes-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    // The parent 0x0a pushes chunk 1 to 0x0b, then refuses it stream 1.
    let out = data_frame(9);
    dump(
        &dir,
        0x0a,
        0x0b,
        3,
        9,
        &[
            (
                t(10),
                Record::Frame {
                    dir: Dir::Out,
                    peer: 1,
                    len: out.len() as u64,
                    body: &out,
                },
            ),
            (
                t(10),
                Record::Tap(TapRecord::new(
                    code("DataSent"),
                    1,
                    1,
                    &[2, 1, t(8).unix_nanos()],
                )),
            ),
            (
                t(12),
                Record::Tap(TapRecord::new(code("Rejected"), 1, 1, &[1])),
            ),
        ],
    );
    let inbound = data_frame(9);
    dump(
        &dir,
        0x0b,
        0x0a,
        9,
        3,
        &[
            (
                t(15),
                Record::Frame {
                    dir: Dir::In,
                    peer: 1,
                    len: inbound.len() as u64,
                    body: &inbound,
                },
            ),
            (
                t(20),
                Record::Tap(TapRecord::new(code("Declined"), 1, 1, &[1])),
            ),
            (
                t(21),
                Record::Tap(TapRecord::new(code("Declined"), 1, 1, &[1])),
            ),
        ],
    );
    let paths = vec![dir.join("0b-0.ppcap"), dir.join("0a-0.ppcap")];
    let mut out = Vec::new();
    let stats = swarm(&paths, &mut out, &mut |e| panic!("{e}")).unwrap();
    let rows: Vec<Value> = String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    let _ = std::fs::remove_dir_all(&dir);

    assert_eq!(
        (stats.nodes, stats.data_joined, stats.decisions_joined),
        (2, 1, 1)
    );
    let times: Vec<i64> = rows.iter().map(|r| r["t"].as_i64().unwrap()).collect();
    assert!(times.windows(2).all(|w| w[0] <= w[1]), "{rows:#?}");
    let data_in = rows
        .iter()
        .find(|r| r["node"] == "0b" && r["type"] == "DATA")
        .unwrap();
    assert_eq!(data_in["sent"], t(10).unix_nanos());
    assert_eq!(data_in["transit"], 5_000_000);
    assert_eq!(data_in["sent_ann"], "push");
    let declined: Vec<&Value> = rows.iter().filter(|r| r["type"] == "Declined").collect();
    assert_eq!(declined.len(), 2);
    assert_eq!(declined[0]["by"]["type"], "Rejected");
    assert_eq!(declined[0]["by"]["node"], "0a");
    assert_eq!(declined[0]["by"]["t"], t(12).unix_nanos());
    // One decision ties to one refusal.
    assert!(declined[1].get("by").is_none(), "{:?}", declined[1]);
}
