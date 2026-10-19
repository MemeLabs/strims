// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Golden vector files: line-based fixtures that pin a crate's wire and
//! store formats. A crate keeps them under `golden/` and replays them from
//! its `tests/golden*.rs`.
//!
//! A blank line, or one starting with `#`, is kept as it is. Any other line
//! is an entry: a name, then fields separated by single spaces. A field is
//! keyed (`key=value`) or positional (`value`, or `=` for an empty value).
//! Several entries may share a name.
//!
//! A test computes every pinned value from inputs in the file (or in the
//! test) and passes it to [`Golden::check`]. Normally a check asserts that
//! the value matches the file. With `STRIMS_GOLDEN=update` in the
//! environment, it rewrites the field instead, and the file is saved when
//! the [`Golden`] drops, unless the test is panicking:
//!
//! ```text
//! STRIMS_GOLDEN=update cargo test -p <crate> --test <test>
//! ```
//!
//! A plain run must pass afterwards. In update mode, [`Golden::entry`]
//! appends an entry the file lacks, a check of a missing field adds it, and
//! [`Golden::remove`] drops entries. Tests in one binary that update the
//! same file take turns: [`Golden::open`] holds a process-wide lock in
//! update mode and reads the file under it, so each sees the others' writes.
//! The lock spans one process only, so an update runs under `cargo test`,
//! which runs a binary's tests in one process (cargo-nextest gives every
//! test a process of its own).
//!
//! [`check_file`] pins a whole file that a recorder writes from scratch.

#![allow(clippy::panic, clippy::expect_used, clippy::indexing_slicing)]

use std::fmt;
use std::path::{Path, PathBuf};
// invariant: test harness threads that update one fixture file take turns.
#[allow(clippy::disallowed_types)]
use std::sync::{Mutex, MutexGuard};

const HINT: &str = "a deliberate format change reruns with STRIMS_GOLDEN=update";

/// Whether this run rewrites golden files instead of checking them.
#[allow(clippy::disallowed_methods)] // the test run's configuration, never a node's
pub fn updating() -> bool {
    std::env::var("STRIMS_GOLDEN").is_ok_and(|v| v == "update")
}

#[allow(clippy::disallowed_types)]
static UPDATE: Mutex<()> = Mutex::new(());

#[allow(clippy::disallowed_types)]
fn update_lock() -> MutexGuard<'static, ()> {
    UPDATE.lock().unwrap_or_else(|e| e.into_inner())
}

/// A field of an entry: its position among the entry's fields, or its key.
#[derive(Clone, Copy, Debug)]
pub enum At<'a> {
    Index(usize),
    Key(&'a str),
}

impl From<usize> for At<'_> {
    fn from(i: usize) -> Self {
        At::Index(i)
    }
}

impl<'a> From<&'a str> for At<'a> {
    fn from(k: &'a str) -> Self {
        At::Key(k)
    }
}

impl fmt::Display for At<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            At::Index(i) => write!(f, "#{i}"),
            At::Key(k) => f.write_str(k),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Line {
    Raw(String),
    Entry(Entry),
}

#[derive(Clone, Debug, PartialEq)]
struct Entry {
    name: String,
    fields: Vec<Field>,
}

#[derive(Clone, Debug, PartialEq)]
struct Field {
    key: Option<String>,
    value: String,
}

fn parse(text: &str) -> Vec<Line> {
    text.lines()
        .map(|l| {
            if l.trim().is_empty() || l.starts_with('#') {
                return Line::Raw(l.to_owned());
            }
            let mut tokens = l.split(' ');
            let name = tokens.next().unwrap_or_default().to_owned();
            let fields = tokens
                .map(|t| match t.split_once('=') {
                    _ if t == "=" => Field {
                        key: None,
                        value: String::new(),
                    },
                    Some((k, v)) if !k.is_empty() => Field {
                        key: Some(k.to_owned()),
                        value: v.to_owned(),
                    },
                    _ => Field {
                        key: None,
                        value: t.to_owned(),
                    },
                })
                .collect();
            Line::Entry(Entry { name, fields })
        })
        .collect()
}

fn render(lines: &[Line]) -> String {
    let mut out = String::new();
    for line in lines {
        match line {
            Line::Raw(s) => out.push_str(s),
            Line::Entry(e) => {
                out.push_str(&e.name);
                for f in &e.fields {
                    out.push(' ');
                    match (&f.key, f.value.is_empty()) {
                        (Some(k), _) => {
                            out.push_str(k);
                            out.push('=');
                            out.push_str(&f.value);
                        }
                        (None, true) => out.push('='),
                        (None, false) => out.push_str(&f.value),
                    }
                }
            }
        }
        out.push('\n');
    }
    out
}

/// Shortens a long value for a failure message, keeping the region where
/// it first differs from `other`.
fn excerpt(s: &str, other: &str) -> String {
    const MAX: usize = 160;
    if s.len() <= MAX {
        return s.to_owned();
    }
    let at = s
        .bytes()
        .zip(other.bytes())
        .position(|(a, b)| a != b)
        .unwrap_or(s.len().min(other.len()));
    let start = at.saturating_sub(MAX / 2);
    let end = (start + MAX).min(s.len());
    format!(
        "…[{start}..{end} of {}] {}…",
        s.len(),
        s.get(start..end).unwrap_or(s)
    )
}

/// An open golden file. See the [crate docs](crate).
pub struct Golden {
    path: PathBuf,
    lines: Vec<Line>,
    update: bool,
    dirty: bool,
    _lock: Option<MutexGuard<'static, ()>>,
}

impl Golden {
    /// Reads the file at `path`, usually
    /// `concat!(env!("CARGO_MANIFEST_DIR"), "/golden/<file>")`.
    pub fn open(path: impl AsRef<Path>) -> Golden {
        let path = path.as_ref().to_owned();
        let update = updating();
        let lock = update.then(update_lock);
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
        Golden {
            lines: parse(&text),
            path,
            update,
            dirty: false,
            _lock: lock,
        }
    }

    /// Whether checks rewrite this file.
    pub fn updating(&self) -> bool {
        self.update
    }

    fn entry_at(&self, i: usize) -> &Entry {
        match self.lines.get(i) {
            Some(Line::Entry(e)) => e,
            _ => panic!("{}:{}: not an entry", self.path.display(), i + 1),
        }
    }

    fn entry_at_mut(&mut self, i: usize) -> &mut Entry {
        match self.lines.get_mut(i) {
            Some(Line::Entry(e)) => e,
            _ => panic!("{}:{}: not an entry", self.path.display(), i + 1),
        }
    }

    /// The entries named `name`, in file order.
    pub fn all(&self, name: &str) -> Vec<usize> {
        self.matching(|n| n == name)
    }

    /// The entries whose names start with `prefix`, in file order.
    pub fn prefixed(&self, prefix: &str) -> Vec<usize> {
        self.matching(|n| n.starts_with(prefix))
    }

    fn matching(&self, f: impl Fn(&str) -> bool) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| match l {
                Line::Entry(e) if f(&e.name) => Some(i),
                _ => None,
            })
            .collect()
    }

    /// The one entry named `name`. In update mode, an entry the file lacks
    /// is appended with no fields.
    #[track_caller]
    pub fn entry(&mut self, name: &str) -> usize {
        match self.all(name)[..] {
            [i] => i,
            [] if self.update => {
                self.lines.push(Line::Entry(Entry {
                    name: name.to_owned(),
                    fields: Vec::new(),
                }));
                self.dirty = true;
                self.lines.len() - 1
            }
            [] => panic!("{}: no entry {name}; {HINT}", self.path.display()),
            _ => panic!("{}: several entries named {name}", self.path.display()),
        }
    }

    /// Removes the entries named `name`. Only update mode may change the
    /// file, so elsewhere their presence fails the test.
    #[track_caller]
    pub fn remove(&mut self, name: &str) {
        let found = !self.all(name).is_empty();
        if found && !self.update {
            panic!("{}: entry {name} is obsolete; {HINT}", self.path.display());
        }
        self.lines
            .retain(|l| !matches!(l, Line::Entry(e) if e.name == name));
        self.dirty |= found;
    }

    /// The entry's name.
    pub fn name(&self, i: usize) -> &str {
        &self.entry_at(i).name
    }

    fn position(e: &Entry, at: At<'_>) -> Option<usize> {
        match at {
            At::Index(n) => (n < e.fields.len()).then_some(n),
            At::Key(k) => e.fields.iter().position(|f| f.key.as_deref() == Some(k)),
        }
    }

    /// The field's value, if the entry has it.
    pub fn opt<'a>(&self, i: usize, at: impl Into<At<'a>>) -> Option<&str> {
        let e = self.entry_at(i);
        Self::position(e, at.into()).map(|p| e.fields[p].value.as_str())
    }

    /// The field's value.
    #[track_caller]
    pub fn str<'a>(&self, i: usize, at: impl Into<At<'a>>) -> &str {
        let at = at.into();
        self.opt(i, at).unwrap_or_else(|| {
            panic!(
                "{}:{} {}: no field {at}",
                self.path.display(),
                i + 1,
                self.name(i)
            )
        })
    }

    /// The field's value, decoded from hex.
    #[track_caller]
    pub fn bytes<'a>(&self, i: usize, at: impl Into<At<'a>>) -> Vec<u8> {
        let at = at.into();
        let s = self.str(i, at);
        hex::decode(s).unwrap_or_else(|e| {
            panic!(
                "{}:{} {}: field {at} isn't hex: {e}",
                self.path.display(),
                i + 1,
                self.name(i)
            )
        })
    }

    /// Checks a computed value against the field, or rewrites the field in
    /// update mode.
    #[track_caller]
    pub fn check<'a>(&mut self, i: usize, at: impl Into<At<'a>>, actual: impl AsRef<str>) {
        let (at, actual) = (at.into(), actual.as_ref());
        assert!(
            !actual.contains(char::is_whitespace),
            "golden values can't contain whitespace: {actual:?}"
        );
        let (path, update) = (self.path.display().to_string(), self.update);
        let e = self.entry_at_mut(i);
        match Self::position(e, at) {
            Some(p) if e.fields[p].value == actual => {}
            Some(p) if update => {
                e.fields[p].value = actual.to_owned();
                self.dirty = true;
            }
            Some(p) => panic!(
                "{path}:{} {}: field {at}\n  pinned:   {}\n  computed: {}\n{HINT}",
                i + 1,
                e.name,
                excerpt(&e.fields[p].value, actual),
                excerpt(actual, &e.fields[p].value),
            ),
            None if update => {
                match at {
                    At::Key(k) => e.fields.push(Field {
                        key: Some(k.to_owned()),
                        value: actual.to_owned(),
                    }),
                    At::Index(n) => {
                        while e.fields.len() <= n {
                            e.fields.push(Field {
                                key: None,
                                value: String::new(),
                            });
                        }
                        e.fields[n].value = actual.to_owned();
                    }
                }
                self.dirty = true;
            }
            None => panic!("{path}:{} {}: no field {at}; {HINT}", i + 1, e.name),
        }
    }

    /// [`check`](Golden::check)s bytes as lowercase hex.
    #[track_caller]
    pub fn check_hex<'a>(&mut self, i: usize, at: impl Into<At<'a>>, actual: impl AsRef<[u8]>) {
        self.check(i, at, hex::encode(actual));
    }
}

impl Drop for Golden {
    fn drop(&mut self) {
        if self.update && self.dirty && !std::thread::panicking() {
            std::fs::write(&self.path, render(&self.lines))
                .unwrap_or_else(|e| panic!("writing {}: {e}", self.path.display()));
        }
    }
}

/// Pins a whole file that a recorder writes from scratch: asserts that
/// `actual` is the file's text, or writes it in update mode.
#[track_caller]
pub fn check_file(path: impl AsRef<Path>, actual: &str) {
    let path = path.as_ref();
    if updating() {
        let _lock = update_lock();
        std::fs::write(path, actual).unwrap_or_else(|e| panic!("writing {}: {e}", path.display()));
        return;
    }
    let pinned =
        std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
    if pinned == actual {
        return;
    }
    let (p, a): (Vec<_>, Vec<_>) = (pinned.lines().collect(), actual.lines().collect());
    let n = p
        .iter()
        .zip(&a)
        .position(|(x, y)| x != y)
        .unwrap_or(p.len().min(a.len()));
    panic!(
        "{}:{}: recorded file differs ({} pinned lines, {} computed)\n  pinned:   {}\n  computed: {}\n{HINT}",
        path.display(),
        n + 1,
        p.len(),
        a.len(),
        p.get(n).map_or_else(
            || "<end>".to_owned(),
            |l| excerpt(l, a.get(n).unwrap_or(&""))
        ),
        a.get(n).map_or_else(
            || "<end>".to_owned(),
            |l| excerpt(l, p.get(n).unwrap_or(&""))
        ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str =
        "# comment\n\nkey_0 0011 = ff\nframe port=1 body= wire=00\nframe port=2 body=ab wire=01\n";

    #[test]
    fn files_round_trip() {
        assert_eq!(render(&parse(SAMPLE)), SAMPLE);
    }

    fn sample() -> Golden {
        Golden {
            path: PathBuf::from("sample"),
            lines: parse(SAMPLE),
            update: false,
            dirty: false,
            _lock: None,
        }
    }

    #[test]
    fn fields_read_by_position_and_key() {
        let g = sample();
        let k = g.all("key_0")[0];
        assert_eq!(g.bytes(k, 0), [0x00, 0x11]);
        assert_eq!(g.str(k, 1), "");
        let frames = g.all("frame");
        assert_eq!(frames.len(), 2);
        assert_eq!(g.str(frames[1], "body"), "ab");
        assert_eq!(g.str(frames[0], "body"), "");
        assert_eq!(g.opt(frames[0], "missing"), None);
        assert_eq!(g.prefixed("fr"), frames);
    }

    #[test]
    fn matching_values_pass() {
        let mut g = sample();
        let k = g.entry("key_0");
        g.check(k, 2, "ff");
        g.check_hex(k, 0, [0x00, 0x11]);
        assert!(!g.dirty);
    }

    #[test]
    #[should_panic(expected = "STRIMS_GOLDEN=update")]
    fn mismatches_fail_outside_update_mode() {
        let mut g = sample();
        let k = g.entry("key_0");
        g.check(k, 2, "fe");
    }

    #[test]
    fn update_mode_rewrites_and_adds() {
        let mut g = sample();
        g.update = true;
        let k = g.entry("key_0");
        g.check(k, 2, "fe");
        g.check(k, 4, "aa");
        let f = g.all("frame")[0];
        g.check(f, "extra", "01");
        let n = g.entry("new");
        g.check(n, "x", "");
        g.remove("frame");
        g.dirty = false; // nothing to write in a unit test
        assert_eq!(
            render(&g.lines),
            "# comment\n\nkey_0 0011 = fe = aa\nnew x=\n"
        );
    }
}
