// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The generated vocabulary ([`strims_wan::vocab`]) is the document's:
//! its text, and every kind the document names, each once.
//! `hack/simlab/tests/test_vocab.py` checks the rest against the generator.

use std::collections::BTreeSet;

use strims_wan::vocab::{self, Kind, Sim};

#[test]
fn the_vocabulary_is_the_documents_text() {
    assert!(
        vocab::DOCUMENT == include_str!("../vocab.toml"),
        "src/vocab.rs is stale: run python3 hack/simlab/vocab.py write"
    );
}

#[test]
fn every_kind_of_the_document_has_one_name() {
    let headers: Vec<&str> = vocab::DOCUMENT
        .lines()
        .filter_map(|l| l.strip_prefix("[kinds.")?.strip_suffix(']'))
        .filter(|k| !k.contains('.'))
        .collect();
    let named: Vec<&str> = Kind::ALL
        .iter()
        .map(|k| k.name())
        .chain(vocab::UNWRITTEN.iter().copied())
        .chain(vocab::RETIRED.iter().copied())
        .chain(vocab::DERIVED.iter().copied())
        .collect();
    let set = |v: &[&'static str]| v.iter().copied().collect::<BTreeSet<_>>();
    assert_eq!(set(&headers).len(), headers.len(), "a kind twice");
    assert_eq!(set(&named).len(), named.len(), "a name twice");
    assert_eq!(set(&headers), set(&named));
}

#[test]
fn a_kind_is_its_names_and_a_sims() {
    for k in Kind::ALL {
        assert_eq!(Kind::from_name(k.name()), Some(k));
        assert!(Sim::ALL.into_iter().any(|s| k.written_by(s)), "{k:?}");
    }
    assert_eq!(Kind::from_name("start"), None);
    assert_eq!(Kind::from_name(""), None);
}
