// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Bindings to the global object, read structurally so they work in a
//! worker and in a window.

use js_sys::Reflect;
use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::*;
use web_sys::{Crypto, IdbFactory, Performance};

#[wasm_bindgen]
extern "C" {
    /// The global object: a `WorkerGlobalScope` or a `Window`.
    pub(crate) type Scope;

    #[wasm_bindgen(method, getter, structural)]
    pub(crate) fn performance(this: &Scope) -> Option<Performance>;

    #[wasm_bindgen(method, getter, structural)]
    pub(crate) fn crypto(this: &Scope) -> Option<Crypto>;

    /// Throws `SecurityError` where storage is denied, e.g. on opaque
    /// origins.
    #[wasm_bindgen(method, getter, structural, catch, js_name = indexedDB)]
    pub(crate) fn indexed_db(this: &Scope) -> Result<Option<IdbFactory>, JsValue>;

    #[wasm_bindgen(js_name = setTimeout)]
    pub(crate) fn set_timeout(handler: &js_sys::Function, ms: i32) -> JsValue;

    #[wasm_bindgen(js_name = clearTimeout)]
    pub(crate) fn clear_timeout(id: &JsValue);
}

/// The global object.
pub(crate) fn scope() -> Scope {
    js_sys::global().unchecked_into()
}

/// The `name` property of a thrown value, e.g. a `DOMException`'s.
pub(crate) fn error_name(e: &JsValue) -> Option<String> {
    property(e, "name")
}

/// A thrown value as text: `name: message` for errors.
pub(crate) fn describe(e: &JsValue) -> String {
    match (property(e, "name"), property(e, "message")) {
        (Some(name), Some(message)) => format!("{name}: {message}"),
        (None, Some(message)) => message,
        _ => e.as_string().unwrap_or_else(|| format!("{e:?}")),
    }
}

fn property(e: &JsValue, key: &str) -> Option<String> {
    if !e.is_object() {
        return None;
    }
    Reflect::get(e, &JsValue::from_str(key)).ok()?.as_string()
}
