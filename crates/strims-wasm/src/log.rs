// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Logging to the console.
//!
//! [`init`] installs a `tracing` subscriber that formats each event on one
//! line, without a timestamp (the console adds its own) or ANSI colours,
//! and writes it with the `console` method for its level: `debug` for
//! TRACE and DEBUG, then `info`, `warn` and `error`.
//!
//! It also installs the panic hook, which writes the panic and its JS
//! stack to `console.error` and then calls the hook set with
//! [`set_fatal_hook`]. A panic aborts the instance after the hook returns.

use std::cell::RefCell;
use std::io;
use std::panic::PanicHookInfo;
use std::rc::Rc;
use std::sync::Once;

use js_sys::Reflect;
use tracing::{Level, Metadata};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::fmt::MakeWriter;
use wasm_bindgen::JsValue;
use web_sys::console;

/// What a panic does once it is logged.
type FatalHook = Rc<dyn Fn(&str)>;

thread_local! {
    static FATAL: RefCell<Option<FatalHook>> = const { RefCell::new(None) };
}

/// Installs the console subscriber and the panic hook. `log_level` is a
/// zap level: −1 or less is debug, 0 info, 1 warn and 2 or more error.
/// Only the first call has an effect.
pub fn init(log_level: i32) {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(level(log_level))
            .without_time()
            .with_ansi(false)
            .with_writer(Console)
            .finish();
        // Nothing else in the worker installs a subscriber.
        let _ = tracing::subscriber::set_global_default(subscriber);
        std::panic::set_hook(Box::new(on_panic));
    });
}

/// Sets what a panic does after it is logged, e.g. telling the window
/// the node has died. It receives the panic message, and replaces any
/// earlier hook.
pub fn set_fatal_hook(hook: impl Fn(&str) + 'static) {
    let old = FATAL.with(|f| f.replace(Some(Rc::new(hook))));
    drop(old);
}

/// The most verbose level a zap level shows.
fn level(log_level: i32) -> LevelFilter {
    match log_level {
        i32::MIN..=-1 => LevelFilter::DEBUG,
        0 => LevelFilter::INFO,
        1 => LevelFilter::WARN,
        _ => LevelFilter::ERROR,
    }
}

fn on_panic(info: &PanicHookInfo<'_>) {
    let message = info.to_string();
    let stack = Reflect::get(&js_sys::Error::new(""), &JsValue::from_str("stack"))
        .ok()
        .and_then(|s| s.as_string())
        .unwrap_or_default();
    console::error_1(&JsValue::from_str(&format!("{message}\n\nStack:\n{stack}")));
    let hook = FATAL
        .try_with(|f| f.try_borrow().ok().and_then(|f| f.clone()))
        .ok()
        .flatten();
    if let Some(hook) = hook {
        hook(&message);
    }
}

/// Makes a [`ConsoleWriter`] per event.
struct Console;

impl<'a> MakeWriter<'a> for Console {
    type Writer = ConsoleWriter;

    fn make_writer(&'a self) -> ConsoleWriter {
        ConsoleWriter::new(Level::INFO)
    }

    fn make_writer_for(&'a self, meta: &Metadata<'_>) -> ConsoleWriter {
        ConsoleWriter::new(*meta.level())
    }
}

/// Collects one formatted event and writes it to the console on drop.
struct ConsoleWriter {
    level: Level,
    line: Vec<u8>,
}

impl ConsoleWriter {
    fn new(level: Level) -> ConsoleWriter {
        ConsoleWriter {
            level,
            line: Vec::new(),
        }
    }
}

impl io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.line.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for ConsoleWriter {
    fn drop(&mut self) {
        if self.line.is_empty() {
            return;
        }
        let line = String::from_utf8_lossy(&self.line);
        let line = JsValue::from_str(line.trim_end());
        match self.level {
            Level::ERROR => console::error_1(&line),
            Level::WARN => console::warn_1(&line),
            Level::INFO => console::info_1(&line),
            _ => console::debug_1(&line),
        }
    }
}
