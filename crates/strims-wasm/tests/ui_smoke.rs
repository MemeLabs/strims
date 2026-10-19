// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The production UI in headless Firefox: a sign-up through its form
//! reaches the signed-in layout, whose activity monitor charts the
//! worker's metrics.
//!
//! The test builds dist/web as `npm run build` does, less its typecheck
//! (`npx webpack --mode production`, so strims-wasm in the release-wasm
//! profile), serves it on 127.0.0.1, and drives Firefox through
//! geckodriver's WebDriver API: it opens `/signup`, fills the sign-up
//! form's name and password, submits it, and waits for the layout's
//! header, which only a signed-in profile sees. Then it opens the
//! header's activity monitor and waits for its heap graph, which shows
//! once the debugger has a metrics snapshot from the worker, under a
//! label that says the snapshot counts the heap.
//!
//! Ignored by default (it launches a browser):
//!
//! ```sh
//! cargo test -p strims-wasm --test ui_smoke -- --ignored
//! ```

#![cfg(not(target_arch = "wasm32"))]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    // the wall clock, for timings
    clippy::disallowed_methods,
    clippy::disallowed_types
)]

mod support;

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde_json::json;
use support::browser::{Site, Unavailable, build};
use support::webdriver::Session;

/// How long the page has to load and show the form: the first load
/// compiles the wasm module.
const FORM_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the sign-up may take, the profile's node start included.
const SIGN_UP_TIMEOUT: Duration = Duration::from_secs(60);
/// How long the activity monitor has to chart its first snapshot.
const METRICS_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
#[ignore = "launches headless Firefox through geckodriver"]
async fn a_sign_up_through_the_production_ui_reaches_the_layout_and_its_activity_monitor() {
    let bundle = match build("web", &["--mode", "production"], "dist/web") {
        Ok(b) => Arc::new(b),
        Err(Unavailable::Skip(why)) => {
            eprintln!("skipping: {why}");
            return;
        }
        Err(Unavailable::Failed(why)) => panic!("{why}"),
    };
    let site = Site::serve(bundle).await;
    let prefs = json!({"devtools.console.stdout.content": true});
    let Some(session) = Session::start(prefs).await else {
        return;
    };

    let started = Instant::now();
    session
        .navigate(&format!("http://{}/signup", site.addr))
        .await;
    let name = session
        .wait_for("form input[name=\"name\"]", FORM_TIMEOUT)
        .await;
    let password = session
        .wait_for("form input[name=\"password\"]", FORM_TIMEOUT)
        .await;
    eprintln!("the sign-up form shows: {:.2?}", started.elapsed());

    let started = Instant::now();
    session.type_into(&name, "dave").await;
    session.type_into(&password, "pw").await;
    let submit = session
        .wait_for("form button.input_button--primary", FORM_TIMEOUT)
        .await;
    session.click(&submit).await;
    session
        .wait_for("header.layout_header", SIGN_UP_TIMEOUT)
        .await;
    let url = session.url().await;
    assert!(
        !url.contains("/signup") && !url.contains("/login"),
        "the signed-in layout at {url}"
    );
    eprintln!(
        "the sign-up reaches the signed-in layout: {:.2?}",
        started.elapsed()
    );

    let started = Instant::now();
    let monitor = session
        .wait_for("button[title=\"Activity monitor\"]", FORM_TIMEOUT)
        .await;
    session.click(&monitor).await;
    let heap = session
        .wait_for(
            ".debugger_graph--memory .debugger_graph__label",
            METRICS_TIMEOUT,
        )
        .await;
    assert_eq!(session.text(&heap).await, "heap bytes");
    eprintln!(
        "the activity monitor charts the worker's heap: {:.2?}",
        started.elapsed()
    );
    session.end().await;
}
