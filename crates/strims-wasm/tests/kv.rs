// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! `IdbKv` in a dedicated worker: the `strims_kv` conformance suite, and
//! how IndexedDB transactions end.
//!
//! Each test's databases share a prefix of their own, and the test
//! deletes them at its end.

#![cfg(target_arch = "wasm32")]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use std::cell::Cell;

use js_sys::{Array, Function, Promise, Reflect};
use strims_base::time::Duration;
use strims_kv::{Error, Kv, ReadTx, WriteTx};
use strims_rpc::Timer;
use strims_wasm::IdbKv;
use strims_wasm::clock::{self, WasmClock};
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_dedicated_worker);

/// A database-name prefix no other test or run shares.
fn unique(test: &str) -> String {
    let seed = strims_wasm::entropy::seed().unwrap();
    let tag: String = seed[..6].iter().map(|b| format!("{b:02x}")).collect();
    format!("strims-wasm-test/{test}/{tag}/")
}

fn indexed_db() -> JsValue {
    Reflect::get(&js_sys::global(), &JsValue::from_str("indexedDB")).unwrap()
}

/// The names of every database, from `indexedDB.databases()`.
async fn database_names() -> Vec<String> {
    let idb = indexed_db();
    let databases: Function = Reflect::get(&idb, &JsValue::from_str("databases"))
        .unwrap()
        .dyn_into()
        .unwrap();
    let list: Promise = databases.call0(&idb).unwrap().dyn_into().unwrap();
    let list = JsFuture::from(list).await.unwrap();
    Array::from(&list)
        .iter()
        .filter_map(|d| {
            Reflect::get(&d, &JsValue::from_str("name"))
                .ok()?
                .as_string()
        })
        .collect()
}

/// Deletes every database whose name starts with `prefix`.
async fn clean_up(prefix: &str) {
    let kv = IdbKv::new("");
    for name in database_names().await {
        if name.starts_with(prefix) {
            kv.delete_table(&name).await.unwrap();
        }
    }
    assert!(
        !database_names().await.iter().any(|n| n.starts_with(prefix)),
        "{prefix} databases remain"
    );
}

async fn write(kv: &IdbKv, table: &str, entries: &[(&str, &[u8])]) {
    let mut tx = kv.begin_write(table).await.unwrap();
    for (k, v) in entries {
        tx.put(k, v).await.unwrap();
    }
    tx.commit().await.unwrap();
}

async fn get(kv: &IdbKv, table: &str, key: &str) -> Option<Vec<u8>> {
    kv.begin_read(table).await.unwrap().get(key).await.unwrap()
}

#[wasm_bindgen_test]
async fn conformance() {
    let prefix = unique("conformance");
    let n = Cell::new(0);
    strims_kv::conformance::run(|| {
        n.set(n.get() + 1);
        IdbKv::new(&format!("{prefix}{}/", n.get()))
    })
    .await;
    clean_up(&prefix).await;
}

#[wasm_bindgen_test]
async fn a_transaction_that_awaits_a_timer_goes_inactive() {
    let prefix = unique("timer");
    let kv = IdbKv::new(&prefix);
    kv.create_table("t").await.unwrap();

    let mut tx = kv.begin_write("t").await.unwrap();
    tx.put("before", b"1").await.unwrap();
    WasmClock
        .sleep_until(clock::now() + Duration::from_millis(10))
        .await;
    assert_eq!(tx.put("after", b"2").await, Err(Error::Inactive));
    assert_eq!(tx.get("before").await, Err(Error::Inactive));
    assert_eq!(tx.commit().await, Err(Error::Inactive));

    // It committed what it wrote before the await.
    assert_eq!(get(&kv, "t", "before").await.as_deref(), Some(&b"1"[..]));
    assert_eq!(get(&kv, "t", "after").await, None);

    let mut tx = kv.begin_read("t").await.unwrap();
    clock::yield_now().await;
    assert_eq!(tx.get("before").await, Err(Error::Inactive));
    assert_eq!(tx.scan("", None, 10).await, Err(Error::Inactive));
    drop(tx);
    clean_up(&prefix).await;
}

#[wasm_bindgen_test]
async fn a_transaction_spans_many_awaited_operations() {
    let prefix = unique("many");
    let kv = IdbKv::new(&prefix);
    kv.create_table("t").await.unwrap();

    let mut tx = kv.begin_write("t").await.unwrap();
    for i in 0..1000u32 {
        tx.put(&format!("k{i:04}"), &i.to_be_bytes()).await.unwrap();
    }
    assert_eq!(
        tx.get("k0999").await.unwrap(),
        Some(999u32.to_be_bytes().to_vec())
    );
    tx.commit().await.unwrap();

    let mut tx = kv.begin_read("t").await.unwrap();
    let all = tx.scan("k", None, usize::MAX).await.unwrap();
    assert_eq!(all.len(), 1000);
    for (i, (k, v)) in (0..1000u32).zip(&all) {
        assert_eq!(*k, format!("k{i:04}"));
        assert_eq!(*v, i.to_be_bytes());
    }
    let page = tx.scan("k", Some("k0499"), 3).await.unwrap();
    let keys: Vec<_> = page.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(keys, ["k0500", "k0501", "k0502"]);
    drop(tx);
    clean_up(&prefix).await;
}

#[wasm_bindgen_test]
async fn dropping_a_write_transaction_discards_it() {
    let prefix = unique("drop");
    let kv = IdbKv::new(&prefix);
    kv.create_table("t").await.unwrap();
    write(&kv, "t", &[("kept", b"1")]).await;

    let mut tx = kv.begin_write("t").await.unwrap();
    tx.put("dropped", b"1").await.unwrap();
    tx.delete("kept").await.unwrap();
    drop(tx);

    assert_eq!(get(&kv, "t", "dropped").await, None);
    assert_eq!(get(&kv, "t", "kept").await.as_deref(), Some(&b"1"[..]));
    clean_up(&prefix).await;
}

#[wasm_bindgen_test]
async fn reading_a_missing_table_creates_no_database() {
    let prefix = unique("missing");
    let kv = IdbKv::new(&prefix);
    assert!(matches!(kv.begin_read("t").await, Err(Error::NoTable)));
    assert!(matches!(kv.begin_write("t").await, Err(Error::NoTable)));
    let names = database_names().await;
    assert!(
        !names.iter().any(|n| n.starts_with(&prefix)),
        "{names:?} has a database under {prefix}"
    );

    // The aborted creations leave nothing in the way of a real one.
    assert!(kv.create_table("t").await.unwrap());
    write(&kv, "t", &[("k", b"v")]).await;
    assert_eq!(get(&kv, "t", "k").await.as_deref(), Some(&b"v"[..]));
    clean_up(&prefix).await;
}

#[wasm_bindgen_test]
async fn deleting_a_table_closes_its_cached_connection() {
    let prefix = unique("delete");
    let kv = IdbKv::new(&prefix);
    let other = IdbKv::new(&prefix);
    kv.create_table("t").await.unwrap();
    write(&kv, "t", &[("k", b"v")]).await;
    // Both stores now hold a connection to the table's database.
    assert_eq!(get(&other, "t", "k").await.as_deref(), Some(&b"v"[..]));

    assert!(database_names().await.contains(&format!("{prefix}t")));

    // Neither connection holds the deletion back, and neither serves the
    // deleted database afterwards.
    kv.delete_table("t").await.unwrap();
    assert!(matches!(kv.begin_read("t").await, Err(Error::NoTable)));
    assert!(matches!(other.begin_read("t").await, Err(Error::NoTable)));
    assert!(!database_names().await.contains(&format!("{prefix}t")));

    assert!(other.create_table("t").await.unwrap());
    assert!(!kv.create_table("t").await.unwrap());
    assert_eq!(get(&kv, "t", "k").await, None);
    other.delete_table("t").await.unwrap();
    assert!(matches!(kv.begin_write("t").await, Err(Error::NoTable)));
    clean_up(&prefix).await;
}

#[wasm_bindgen_test]
async fn a_transaction_begun_inside_another_does_not_wait_on_it() {
    let prefix = unique("nested");
    let kv = IdbKv::new(&prefix);
    kv.create_table("t").await.unwrap();
    kv.create_table("u").await.unwrap();

    let mut outer = kv.begin_write("t").await.unwrap();
    outer.put("outer", b"1").await.unwrap();
    let mut inner = kv.begin_write("t").await.unwrap();
    inner.put("inner", b"2").await.unwrap();
    inner.commit().await.unwrap();
    assert_eq!(outer.put("late", b"3").await, Err(Error::Inactive));
    drop(outer);
    assert_eq!(get(&kv, "t", "outer").await.as_deref(), Some(&b"1"[..]));
    assert_eq!(get(&kv, "t", "inner").await.as_deref(), Some(&b"2"[..]));

    let mut outer = kv.begin_write("t").await.unwrap();
    outer.put("k", b"1").await.unwrap();
    kv.delete_table("t").await.unwrap();
    assert_eq!(outer.put("k", b"2").await, Err(Error::Inactive));
    drop(outer);
    assert!(matches!(kv.begin_read("t").await, Err(Error::NoTable)));
    clean_up(&prefix).await;
}
