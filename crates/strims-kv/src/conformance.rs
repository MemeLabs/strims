// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The behaviour every [`Kv`] backend shares, as a test suite.
//!
//! [`run`] panics at the first failed check. Backends whose operations
//! are ready immediately also pass [`run_immediate`], and can drive both
//! with `now_or_never`.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use crate::{Error, Kv, ReadTx, WriteTx};

/// Runs every check, each against a fresh store from `make`.
pub async fn run<K: Kv>(make: impl Fn() -> K) {
    tables(&make()).await;
    get_put_delete(&make()).await;
    commit_and_rollback(&make()).await;
    scans(&make()).await;
    scans_see_uncommitted_writes(&make()).await;
}

/// Runs the checks for backends whose operations complete immediately:
/// [`run`], and [`Error::Busy`] for any transaction or table operation
/// begun while a transaction is open.
pub async fn run_immediate<K: Kv>(make: impl Fn() -> K) {
    run(&make).await;
    nested_transactions_are_busy(&make()).await;
}

async fn write<K: Kv>(kv: &K, table: &str, entries: &[(&str, &[u8])]) {
    let mut tx = kv.begin_write(table).await.unwrap();
    for (k, v) in entries {
        tx.put(k, v).await.unwrap();
    }
    tx.commit().await.unwrap();
}

async fn get<K: Kv>(kv: &K, table: &str, key: &str) -> Option<Vec<u8>> {
    kv.begin_read(table).await.unwrap().get(key).await.unwrap()
}

fn keys(entries: &[(String, Vec<u8>)]) -> Vec<&str> {
    entries.iter().map(|(k, _)| k.as_str()).collect()
}

async fn tables<K: Kv>(kv: &K) {
    assert!(matches!(kv.begin_read("a").await, Err(Error::NoTable)));
    assert!(matches!(kv.begin_write("a").await, Err(Error::NoTable)));
    assert!(kv.create_table("a").await.unwrap());
    assert!(!kv.create_table("a").await.unwrap());
    assert!(kv.create_table("b").await.unwrap());

    write(kv, "a", &[("k", b"in a")]).await;
    write(kv, "b", &[("k", b"in b")]).await;
    assert_eq!(get(kv, "a", "k").await.as_deref(), Some(&b"in a"[..]));
    assert_eq!(get(kv, "b", "k").await.as_deref(), Some(&b"in b"[..]));

    kv.delete_table("a").await.unwrap();
    kv.delete_table("a").await.unwrap();
    assert!(matches!(kv.begin_read("a").await, Err(Error::NoTable)));
    assert!(kv.create_table("a").await.unwrap());
    assert_eq!(get(kv, "a", "k").await, None);
    assert_eq!(get(kv, "b", "k").await.as_deref(), Some(&b"in b"[..]));
}

async fn get_put_delete<K: Kv>(kv: &K) {
    kv.create_table("t").await.unwrap();
    assert_eq!(get(kv, "t", "k").await, None);

    let mut tx = kv.begin_write("t").await.unwrap();
    tx.put("k", b"1").await.unwrap();
    tx.put("empty", b"").await.unwrap();
    assert_eq!(tx.get("k").await.unwrap().as_deref(), Some(&b"1"[..]));
    tx.put("k", b"2").await.unwrap();
    assert_eq!(tx.get("k").await.unwrap().as_deref(), Some(&b"2"[..]));
    tx.commit().await.unwrap();
    assert_eq!(get(kv, "t", "k").await.as_deref(), Some(&b"2"[..]));
    assert_eq!(get(kv, "t", "empty").await.as_deref(), Some(&b""[..]));

    let mut tx = kv.begin_write("t").await.unwrap();
    tx.delete("k").await.unwrap();
    tx.delete("missing").await.unwrap();
    assert_eq!(tx.get("k").await.unwrap(), None);
    tx.put("k", b"3").await.unwrap();
    tx.delete("k").await.unwrap();
    tx.commit().await.unwrap();
    assert_eq!(get(kv, "t", "k").await, None);
}

async fn commit_and_rollback<K: Kv>(kv: &K) {
    kv.create_table("t").await.unwrap();
    write(kv, "t", &[("kept", b"1")]).await;

    let mut tx = kv.begin_write("t").await.unwrap();
    tx.put("dropped", b"1").await.unwrap();
    tx.delete("kept").await.unwrap();
    drop(tx);

    assert_eq!(get(kv, "t", "dropped").await, None);
    assert_eq!(get(kv, "t", "kept").await.as_deref(), Some(&b"1"[..]));
}

async fn scans<K: Kv>(kv: &K) {
    kv.create_table("t").await.unwrap();
    write(
        kv,
        "t",
        &[
            ("a", b"0"),
            ("a:1", b"1"),
            ("a:2", b"2"),
            ("a:3", b"3"),
            ("a;", b"4"),
            ("b:1", b"5"),
            ("a:", b"6"),
        ],
    )
    .await;

    let mut tx = kv.begin_read("t").await.unwrap();
    let all = tx.scan("a:", None, usize::MAX).await.unwrap();
    assert_eq!(keys(&all), ["a:", "a:1", "a:2", "a:3"]);
    assert_eq!(all[1].1, b"1");

    let after = tx.scan("a:", Some("a:1"), usize::MAX).await.unwrap();
    assert_eq!(keys(&after), ["a:2", "a:3"]);
    let before_prefix = tx.scan("a:", Some("a"), usize::MAX).await.unwrap();
    assert_eq!(keys(&before_prefix), ["a:", "a:1", "a:2", "a:3"]);
    let past_end = tx.scan("a:", Some("a:9"), usize::MAX).await.unwrap();
    assert!(past_end.is_empty());

    let limited = tx.scan("a:", None, 2).await.unwrap();
    assert_eq!(keys(&limited), ["a:", "a:1"]);
    assert!(tx.scan("a:", None, 0).await.unwrap().is_empty());

    let everything = tx.scan("", None, usize::MAX).await.unwrap();
    assert_eq!(everything.len(), 7);
    assert!(tx.scan("c", None, usize::MAX).await.unwrap().is_empty());
}

async fn scans_see_uncommitted_writes<K: Kv>(kv: &K) {
    kv.create_table("t").await.unwrap();
    write(kv, "t", &[("p:1", b"1"), ("p:3", b"3"), ("p:5", b"5")]).await;

    let mut tx = kv.begin_write("t").await.unwrap();
    tx.put("p:2", b"2").await.unwrap();
    tx.put("p:3", b"three").await.unwrap();
    tx.delete("p:5").await.unwrap();
    tx.put("p:6", b"6").await.unwrap();
    tx.delete("p:7").await.unwrap();

    let all = tx.scan("p:", None, usize::MAX).await.unwrap();
    assert_eq!(keys(&all), ["p:1", "p:2", "p:3", "p:6"]);
    assert_eq!(all[2].1, b"three");
    let limited = tx.scan("p:", Some("p:1"), 2).await.unwrap();
    assert_eq!(keys(&limited), ["p:2", "p:3"]);
    tx.commit().await.unwrap();

    let mut tx = kv.begin_read("t").await.unwrap();
    let all = tx.scan("p:", None, usize::MAX).await.unwrap();
    assert_eq!(keys(&all), ["p:1", "p:2", "p:3", "p:6"]);
}

async fn nested_transactions_are_busy<K: Kv>(kv: &K) {
    kv.create_table("t").await.unwrap();
    kv.create_table("u").await.unwrap();

    let tx = kv.begin_write("t").await.unwrap();
    assert!(matches!(kv.begin_read("t").await, Err(Error::Busy)));
    assert!(matches!(kv.begin_write("u").await, Err(Error::Busy)));
    assert!(matches!(kv.create_table("v").await, Err(Error::Busy)));
    assert!(matches!(kv.delete_table("u").await, Err(Error::Busy)));
    drop(tx);

    let tx = kv.begin_read("t").await.unwrap();
    assert!(matches!(kv.begin_write("t").await, Err(Error::Busy)));
    drop(tx);

    let tx = kv.begin_write("t").await.unwrap();
    tx.commit().await.unwrap();
    kv.begin_write("u").await.unwrap().commit().await.unwrap();
}
