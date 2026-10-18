// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Replays the recorded vectors in `golden/vectors.txt`.
//!
//! Every closest lookup must match brute force. The recorded answer is
//! compared too: equal where the vector marks it exact, different where it
//! doesn't.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic
)]

use std::collections::BTreeMap;

use strims_kademlia::{ID, Insert, KBucket};

const VECTORS: &str = include_str!("../golden/vectors.txt");

fn fields(line: &str) -> (&str, BTreeMap<&str, &str>) {
    let mut it = line.split(' ');
    let kind = it.next().unwrap();
    let kv = it.map(|f| f.split_once('=').unwrap()).collect();
    (kind, kv)
}

fn id(h: &str) -> ID {
    let b: [u8; 32] = hex::decode(h).unwrap().try_into().unwrap();
    ID::from_bytes(&b)
}

fn ids(list: &str) -> Vec<ID> {
    if list == "-" {
        return Vec::new();
    }
    list.split(',').map(id).collect()
}

#[test]
fn golden() {
    let mut table: Option<KBucket<ID>> = None;
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut inexact = 0;

    for line in VECTORS.lines().filter(|l| !l.starts_with('#')) {
        let (kind, f) = fields(line);
        *counts.entry(kind).or_default() += 1;
        match kind {
            "id" => {
                let bytes = hex::decode(f["bytes"]).unwrap();
                let (v, n) = ID::unmarshal(&bytes).unwrap();
                assert_eq!(n, 32);
                let words: Vec<u64> = f["words"]
                    .split(',')
                    .map(|w| u64::from_str_radix(w, 16).unwrap())
                    .collect();
                assert_eq!(v.0.to_vec(), words);
                assert_eq!(v.to_string(), f["string"]);
                assert_eq!(v.marshal_json().as_slice(), f["json"].as_bytes());
                assert_eq!(ID::unmarshal_json(f["json"].as_bytes()), Ok(v));
                let mut out = [0u8; 32];
                v.marshal(&mut out).unwrap();
                assert_eq!(out.as_slice(), bytes.as_slice());
            }
            "xor" => {
                let (a, b) = (id(f["a"]), id(f["b"]));
                assert_eq!(a ^ b, id(f["out"]));
                assert_eq!(a.less(&b), f["less"] == "true");
            }
            "kbucket" => {
                table = Some(KBucket::new(id(f["self"]), f["k"].parse().unwrap()));
            }
            "insert" => {
                let t = table.as_mut().unwrap();
                let n = id(f["id"]);
                let res = t.insert(n);
                assert_eq!(res.inserted(), f["ok"] == "true", "{line}");
                match f["evicted"] {
                    "-" => assert!(!matches!(res, Insert::Evicted(_)), "{line}"),
                    e => assert_eq!(res, Insert::Evicted(id(e)), "{line}"),
                }
            }
            "remove" => {
                let t = table.as_mut().unwrap();
                assert_eq!(
                    t.remove(&id(f["id"])).is_some(),
                    f["ok"] == "true",
                    "{line}"
                );
            }
            "slice" => {
                let t = table.as_ref().unwrap();
                assert_eq!(t.iter().copied().collect::<Vec<_>>(), ids(f["ids"]));
            }
            "closest" => {
                let t = table.as_ref().unwrap();
                let target = id(f["target"]);
                let n: usize = f["n"].parse().unwrap();
                let mut out = Vec::new();
                t.closest(&target, n, &mut out);
                let got: Vec<ID> = out.into_iter().copied().collect();
                let mut brute: Vec<ID> = t.iter().copied().collect();
                brute.sort_by_key(|e| *e ^ target);
                brute.truncate(n);
                assert_eq!(got, brute, "{line}");
                if f["exact"] == "true" {
                    assert_eq!(got, ids(f["ids"]), "{line}");
                } else {
                    assert_ne!(got, ids(f["ids"]), "{line}");
                    inexact += 1;
                }
            }
            other => panic!("unknown golden line kind {other}"),
        }
    }

    // Guard against an empty or truncated vectors file.
    assert_eq!(counts["id"], 11);
    assert_eq!(counts["insert"], 400);
    assert_eq!(counts["closest"], 120);
    assert!(inexact > 0);
}
