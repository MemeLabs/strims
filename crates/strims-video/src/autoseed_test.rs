// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

#![allow(clippy::unwrap_used, clippy::indexing_slicing)]

use strims_ppspp::Uri;
use strims_proto::network::v1::directory::listing::{Chat, Media};

use super::*;
use crate::capture_options;

const NET: &[u8] = b"net-a";
const OTHER_NET: &[u8] = b"net-b";

fn rule(id: u64, network_key: &[u8], swarm: u8, salt: &[u8]) -> (u64, Rule) {
    (
        id,
        Rule {
            id,
            network_key: network_key.to_vec(),
            swarm_id: vec![swarm; 32],
            salt: salt.to_vec(),
            label: format!("rule {id}"),
        },
    )
}

fn media_with(swarm: u8, options: &SwarmOptions) -> Listing {
    let uri = Uri::new(SwarmId::new(&[swarm; 32]), options.uri_options());
    Listing {
        content: Some(Content::Media(Media {
            mime_type: "video/mp4".into(),
            swarm_uri: uri.to_string(),
        })),
    }
}

fn media(swarm: u8) -> Listing {
    media_with(swarm, &capture_options())
}

/// The index of `listings`, by network key and listing id, for `rules`.
fn index(rules: &BTreeMap<u64, Rule>, listings: &[(&[u8], u64, Listing)]) -> Index {
    let mut index = Index::new(rules.values());
    for (key, id, l) in listings {
        index.update(&EgressSettings::default(), key, *id, l);
    }
    index
}

/// The listing each rule wants, by rule id.
fn wanted(wants: &BTreeMap<u64, Want>) -> Vec<(u64, u64)> {
    wants.iter().map(|(r, w)| (*r, w.listing_id)).collect()
}

#[test]
fn rules_match_media_listings_of_their_swarm_on_their_network() {
    let rules = BTreeMap::from([
        rule(1, NET, 1, b""),
        rule(2, NET, 2, b"salted"),
        rule(3, OTHER_NET, 3, b""),
        rule(4, NET, 4, b""),
    ]);
    let chat = Listing {
        content: Some(Content::Chat(Chat {
            key: vec![4; 32],
            name: "chat".into(),
        })),
    };
    let ix = index(
        &rules,
        &[
            (NET, 10, media(1)),
            (NET, 11, media(2)),
            // On another network than its rule's.
            (NET, 12, media(3)),
            (NET, 13, chat),
        ],
    );
    let wants = plan(true, &rules, &ix, 8);
    assert_eq!(wanted(&wants), [(1, 10), (2, 11)]);
    let w = &wants[&2];
    assert_eq!(w.network_key, NET);
    assert_eq!(w.salt, b"salted");
    assert_eq!(
        w.transfer_id(),
        TransferId::new(&SwarmId::new(&[2; 32]), b"salted")
    );
    assert_eq!(
        w.options.live_window,
        EgressSettings::default().window_bytes / capture_options().chunk_size
    );
}

#[test]
fn a_disabled_config_wants_no_seeds() {
    let rules = BTreeMap::from([rule(1, NET, 1, b"")]);
    let ix = index(&rules, &[(NET, 10, media(1))]);
    assert!(plan(false, &rules, &ix, 8).is_empty());
    assert_eq!(plan(true, &rules, &ix, 8).len(), 1);
}

#[test]
fn the_lowest_listing_id_wins() {
    let rules = BTreeMap::from([rule(1, NET, 1, b"")]);
    let mut ix = index(&rules, &[(NET, 30, media(1)), (NET, 20, media(1))]);
    assert_eq!(wanted(&plan(true, &rules, &ix, 8)), [(1, 20)]);
    assert!(ix.remove(NET, 20));
    assert_eq!(wanted(&plan(true, &rules, &ix, 8)), [(1, 30)]);
    assert!(!ix.remove(NET, 20));
}

#[test]
fn max_seeds_holds_from_the_lowest_rule_id() {
    let rules: BTreeMap<u64, Rule> = (1..=5).map(|i| rule(i, NET, i as u8, b"")).collect();
    // Rule 2's swarm isn't listed, so it takes no place.
    let listings: Vec<(&[u8], u64, Listing)> = [1u8, 3, 4, 5]
        .iter()
        .map(|i| (NET, 100 + u64::from(*i), media(*i)))
        .collect();
    let ix = index(&rules, &listings);
    assert_eq!(
        wanted(&plan(true, &rules, &ix, 3)),
        [(1, 101), (3, 103), (4, 104)]
    );
}

#[test]
fn listings_with_bad_options_are_skipped() {
    let rules = BTreeMap::from([rule(1, NET, 1, b"")]);
    let tiny_chunks = SwarmOptions {
        chunk_size: 64,
        ..capture_options()
    };
    let bad_uri = Listing {
        content: Some(Content::Media(Media {
            mime_type: "video/mp4".into(),
            swarm_uri: "magnet:?xt=urn:ppspp:@@@".into(),
        })),
    };
    let ix = index(
        &rules,
        &[
            (NET, 10, media_with(1, &tiny_chunks)),
            (NET, 11, bad_uri),
            (NET, 12, media(1)),
        ],
    );
    assert_eq!(wanted(&plan(true, &rules, &ix, 8)), [(1, 12)]);
}

#[test]
fn the_index_holds_only_listings_rules_name() {
    let rules = BTreeMap::from([rule(1, NET, 1, b"")]);
    let egress = EgressSettings::default();
    let mut ix = Index::new(rules.values());
    assert!(!ix.update(&egress, OTHER_NET, 10, &media(1)));
    assert!(!ix.update(&egress, NET, 10, &media(2)));
    assert!(ix.update(&egress, NET, 10, &media(1)));
    assert!(!ix.update(&egress, NET, 10, &media(1)));
    // A listing that changes to another swarm leaves the index.
    assert!(ix.update(&egress, NET, 10, &media(2)));
    assert!(!ix.clear(NET));
    assert!(ix.update(&egress, NET, 11, &media(1)));
    assert!(ix.clear(NET));
    assert!(plan(true, &rules, &ix, 8).is_empty());
}

#[test]
fn rules_are_bounded() {
    let s = AutoseedSettings::default();
    let ok = rule(1, &[1; MAX_NETWORK_KEY_BYTES], 1, &[2; 64]).1;
    let ok = Rule {
        label: "l".repeat(s.max_label_bytes),
        swarm_id: vec![3; MAX_SWARM_ID_BYTES],
        ..ok
    };
    assert_eq!(check_rule(&s, &ok), Ok(()));
    assert_eq!(
        check_rule(
            &s,
            &Rule {
                salt: Vec::new(),
                label: String::new(),
                ..ok.clone()
            }
        ),
        Ok(())
    );
    for (bad, field) in [
        (
            Rule {
                network_key: Vec::new(),
                ..ok.clone()
            },
            "network key",
        ),
        (
            Rule {
                network_key: vec![1; MAX_NETWORK_KEY_BYTES + 1],
                ..ok.clone()
            },
            "network key",
        ),
        (
            Rule {
                swarm_id: Vec::new(),
                ..ok.clone()
            },
            "swarm id",
        ),
        (
            Rule {
                swarm_id: vec![1; MAX_SWARM_ID_BYTES + 1],
                ..ok.clone()
            },
            "swarm id",
        ),
        (
            Rule {
                salt: vec![1; s.max_salt_bytes + 1],
                ..ok.clone()
            },
            "salt",
        ),
        (
            Rule {
                label: "l".repeat(s.max_label_bytes + 1),
                ..ok.clone()
            },
            "label",
        ),
    ] {
        assert_eq!(check_rule(&s, &bad), Err(VideoError::AutoseedRule(field)));
    }
}
