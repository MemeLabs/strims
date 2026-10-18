// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The options the window starts the worker with.
//!
//! The window sends a plain object (`WasmNodeOptions` in `src/web/wasm.ts`):
//!
//! | field | type | default |
//! |---|---|---|
//! | `logLevel` | a zap level (see [`crate::log::init`]) | 0, info |
//! | `allowInsecureWs` | whether the node dials `ws://` | false |
//! | `iceServers` | ICE server URLs | the core's defaults |
//! | `e2e` | start the bare node of the browser link tests | false |
//!
//! Parsing never fails. A field of the wrong type takes its default, and
//! [`Options::parse`] reports it with every unknown field, for the caller
//! to log once logging is up. Close types are taken: a number or numeric
//! string for `logLevel`, `"true"`/`"false"` or 0/1 for the flags, and one
//! string for `iceServers`. `undefined` and `null` are absent.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::{JsCast, JsValue};

/// The worker's options; see the [module docs](self).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    /// A zap level: −1 debug, 0 info, 1 warn, 2 error.
    pub log_level: i32,
    /// Whether the node dials `ws://` links.
    pub allow_insecure_ws: bool,
    /// The ICE servers of the node's peer connections; `None` is the
    /// core's defaults, an empty list none.
    pub ice_servers: Option<Vec<String>>,
    /// Whether to start the bare node of the browser link tests.
    pub e2e: bool,
}

const FIELDS: [&str; 4] = ["logLevel", "allowInsecureWs", "iceServers", "e2e"];

impl Options {
    /// The options in `value`, and what was ignored in it, one line each.
    pub fn parse(value: &JsValue) -> (Options, Vec<String>) {
        let mut ignored = Vec::new();
        let mut options = Options::default();
        if value.is_undefined() || value.is_null() {
            return (options, ignored);
        }
        if !value.is_object() {
            ignored.push(format!("options: not an object: {}", show(value)));
            return (options, ignored);
        }
        if let Some(v) = field(value, "logLevel") {
            match log_level(&v) {
                Some(l) => options.log_level = l,
                None => ignored.push(format!("logLevel: not a number: {}", show(&v))),
            }
        }
        if let Some(v) = field(value, "allowInsecureWs") {
            match flag(&v) {
                Some(b) => options.allow_insecure_ws = b,
                None => ignored.push(format!("allowInsecureWs: not a boolean: {}", show(&v))),
            }
        }
        if let Some(v) = field(value, "iceServers") {
            options.ice_servers = ice_servers(&v, &mut ignored);
        }
        if let Some(v) = field(value, "e2e") {
            match flag(&v) {
                Some(b) => options.e2e = b,
                None => ignored.push(format!("e2e: not a boolean: {}", show(&v))),
            }
        }
        if let Some(o) = value.dyn_ref::<Object>() {
            for key in Object::keys(o).iter() {
                let key = key.as_string().unwrap_or_default();
                if !FIELDS.contains(&key.as_str()) {
                    ignored.push(format!("{key}: unknown option"));
                }
            }
        }
        (options, ignored)
    }
}

/// `value[key]`, or `None` when it is absent, `undefined` or `null`.
fn field(value: &JsValue, key: &str) -> Option<JsValue> {
    Reflect::get(value, &JsValue::from_str(key))
        .ok()
        .filter(|v| !v.is_undefined() && !v.is_null())
}

fn log_level(v: &JsValue) -> Option<i32> {
    let n = match v.as_f64() {
        Some(n) => n,
        None => v.as_string()?.trim().parse::<f64>().ok()?,
    };
    // `as` saturates, so huge levels are still the extremes.
    n.is_finite().then_some(n.trunc() as i32)
}

fn flag(v: &JsValue) -> Option<bool> {
    if let Some(b) = v.as_bool() {
        return Some(b);
    }
    if let Some(n) = v.as_f64() {
        return if n == 0.0 {
            Some(false)
        } else if n == 1.0 {
            Some(true)
        } else {
            None
        };
    }
    match v.as_string()?.trim() {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// The ICE servers in `v`: an array's strings, or one string. Other
/// entries are dropped.
fn ice_servers(v: &JsValue, ignored: &mut Vec<String>) -> Option<Vec<String>> {
    if let Some(s) = v.as_string() {
        return Some(vec![s]);
    }
    let Some(list) = v.dyn_ref::<Array>() else {
        ignored.push(format!("iceServers: not a list: {}", show(v)));
        return None;
    };
    let mut servers = Vec::new();
    for (i, entry) in list.iter().enumerate() {
        match entry.as_string() {
            Some(s) => servers.push(s),
            None => ignored.push(format!("iceServers[{i}]: not a string: {}", show(&entry))),
        }
    }
    Some(servers)
}

/// A short rendering of an ignored value.
fn show(v: &JsValue) -> String {
    const MAX: usize = 64;
    let s = match v.as_string() {
        Some(s) => format!("{s:?}"),
        None => match v.js_typeof().as_string() {
            Some(t) if t == "object" || t == "function" => t,
            _ => format!("{v:?}"),
        },
    };
    match s.char_indices().nth(MAX) {
        Some((end, _)) => format!("{}…", s.get(..end).unwrap_or_default()),
        None => s,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use wasm_bindgen_test::wasm_bindgen_test;

    use super::*;

    fn object(fields: &[(&str, JsValue)]) -> JsValue {
        let o = Object::new();
        for (k, v) in fields {
            Reflect::set(&o, &JsValue::from_str(k), v).unwrap();
        }
        o.into()
    }

    fn strings(list: &[&str]) -> JsValue {
        list.iter()
            .map(|s| JsValue::from_str(s))
            .collect::<Array>()
            .into()
    }

    #[wasm_bindgen_test]
    fn the_window_options_parse() {
        let (o, ignored) = Options::parse(&object(&[
            ("logLevel", (-1).into()),
            ("allowInsecureWs", true.into()),
            ("iceServers", strings(&["stun:stun.example.com:3478"])),
        ]));
        assert_eq!(
            o,
            Options {
                log_level: -1,
                allow_insecure_ws: true,
                ice_servers: Some(vec!["stun:stun.example.com:3478".to_owned()]),
                e2e: false,
            }
        );
        assert!(ignored.is_empty(), "{ignored:?}");
        let (o, _) = Options::parse(&object(&[
            ("iceServers", strings(&[])),
            ("e2e", true.into()),
        ]));
        assert_eq!((o.ice_servers, o.e2e), (Some(vec![]), true));
    }

    #[wasm_bindgen_test]
    fn absent_options_take_their_defaults() {
        for v in [JsValue::UNDEFINED, JsValue::NULL, object(&[])] {
            assert_eq!(Options::parse(&v), (Options::default(), vec![]));
        }
        let (o, ignored) = Options::parse(&object(&[
            ("logLevel", JsValue::NULL),
            ("iceServers", JsValue::UNDEFINED),
        ]));
        assert_eq!((o, ignored), (Options::default(), vec![]));
    }

    #[wasm_bindgen_test]
    fn close_types_are_taken() {
        let (o, ignored) = Options::parse(&object(&[
            ("logLevel", " 2 ".into()),
            ("allowInsecureWs", 1.into()),
            ("iceServers", "stun:a".into()),
            ("e2e", "true".into()),
        ]));
        assert_eq!(
            o,
            Options {
                log_level: 2,
                allow_insecure_ws: true,
                ice_servers: Some(vec!["stun:a".to_owned()]),
                e2e: true,
            }
        );
        assert!(ignored.is_empty(), "{ignored:?}");
        let (o, _) = Options::parse(&object(&[("logLevel", 1.9e12.into())]));
        assert_eq!(o.log_level, i32::MAX);
    }

    #[wasm_bindgen_test]
    fn wrong_types_are_ignored_and_reported() {
        let mixed: Array = [JsValue::from_str("stun:a"), 7.into(), JsValue::NULL]
            .into_iter()
            .collect();
        let (o, ignored) = Options::parse(&object(&[
            ("logLevel", "loud".into()),
            ("allowInsecureWs", 2.into()),
            ("iceServers", mixed.into()),
            ("e2e", object(&[])),
            ("extra", 1.into()),
        ]));
        assert_eq!(
            o,
            Options {
                ice_servers: Some(vec!["stun:a".to_owned()]),
                ..Options::default()
            }
        );
        let fields: Vec<&str> = ignored
            .iter()
            .map(|l| l.split(':').next().unwrap())
            .collect();
        assert_eq!(
            fields,
            [
                "logLevel",
                "allowInsecureWs",
                "iceServers[1]",
                "iceServers[2]",
                "e2e",
                "extra"
            ]
        );
        let (o, ignored) = Options::parse(&object(&[("iceServers", 5.into())]));
        assert_eq!((o.ice_servers, ignored.len()), (None, 1));
        let (o, ignored) = Options::parse(&"x".into());
        assert_eq!((o, ignored.len()), (Options::default(), 1));
    }
}
