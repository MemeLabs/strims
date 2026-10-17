// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Moderation records: who may moderate, and how a moderation request
//! updates a listing's or user's record.
//!
//! The server's control reads the caller's [`UserRecord`] and checks
//! [`is_moderator`], merges the request into the target's record in a
//! store transaction, then tells the [`Server`](crate::Server) the merged
//! moderation. A request field that is unset leaves the record's field as
//! it is; a record without moderation has none set.

use strims_dao::schema::directory::{new_listing_record, new_user_record};
use strims_proto::network::v1::directory::{
    Listing, ListingModeration, ListingRecord, UserModeration, UserRecord,
};

/// Whether the user with record `r` may moderate.
pub fn is_moderator(r: Option<&UserRecord>) -> bool {
    moderation(r).and_then(|m| m.is_moderator) == Some(true)
}

/// Whether the user with record `r` may grant moderation.
pub fn is_admin(r: Option<&UserRecord>) -> bool {
    moderation(r).and_then(|m| m.is_admin) == Some(true)
}

fn moderation(r: Option<&UserRecord>) -> Option<&UserModeration> {
    r.and_then(|r| r.moderation.as_ref())
}

/// The record of `listing` after moderation `req`: `existing` with each
/// field `req` sets, or a new record with id `new_id` holding `req`.
pub fn merge_listing(
    existing: Option<ListingRecord>,
    req: Option<&ListingModeration>,
    new_id: u64,
    network_id: u64,
    listing: &Listing,
) -> ListingRecord {
    let Some(mut r) = existing else {
        return new_listing_record(new_id, network_id, listing.clone(), req.cloned());
    };
    if let Some(req) = req {
        let mut m = r.moderation.take().unwrap_or_default();
        set(&mut m.is_mature, &req.is_mature);
        set(&mut m.is_banned, &req.is_banned);
        set(&mut m.category, &req.category);
        r.moderation = Some(m);
    }
    r
}

/// The record of the user with `peer_key` after moderation `req`:
/// `existing` with each field `req` sets, or a new record with id
/// `new_id`. `is_moderator` and `is_admin` change only when the caller is
/// an admin.
pub fn merge_user(
    existing: Option<UserRecord>,
    req: Option<&UserModeration>,
    caller_is_admin: bool,
    new_id: u64,
    network_id: u64,
    peer_key: &[u8],
) -> UserRecord {
    let mut r = existing
        .unwrap_or_else(|| new_user_record(new_id, network_id, peer_key, Default::default()));
    if let Some(req) = req {
        let mut m = r.moderation.take().unwrap_or_default();
        set(&mut m.disable_join, &req.disable_join);
        set(&mut m.disable_publish, &req.disable_publish);
        if caller_is_admin {
            set(&mut m.is_moderator, &req.is_moderator);
            set(&mut m.is_admin, &req.is_admin);
        }
        r.moderation = Some(m);
    }
    r
}

fn set<T: Clone>(field: &mut Option<T>, v: &Option<T>) {
    if v.is_some() {
        field.clone_from(v);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use strims_proto::network::v1::directory::listing::{Chat, Content};

    fn chat() -> Listing {
        Listing {
            content: Some(Content::Chat(Chat {
                key: vec![1; 32],
                name: "c".into(),
            })),
        }
    }

    fn listing_record(m: Option<ListingModeration>) -> ListingRecord {
        new_listing_record(5, 9, chat(), m)
    }

    #[test]
    fn a_first_listing_moderation_creates_a_record_with_a_new_id() {
        let req = ListingModeration {
            is_banned: Some(true),
            ..Default::default()
        };
        let r = merge_listing(None, Some(&req), 77, 9, &chat());
        assert_eq!(r.id, 77);
        assert_eq!(r.network_id, 9);
        assert_eq!(r.listing, Some(chat()));
        assert_eq!(r.moderation, Some(req));
    }

    #[test]
    fn a_listing_moderation_sets_only_its_fields() {
        let existing = listing_record(Some(ListingModeration {
            is_mature: Some(true),
            category: Some("games".into()),
            ..Default::default()
        }));
        let req = ListingModeration {
            is_banned: Some(true),
            category: Some(String::new()),
            ..Default::default()
        };
        let r = merge_listing(Some(existing), Some(&req), 77, 9, &chat());
        assert_eq!(r.id, 5);
        assert_eq!(
            r.moderation,
            Some(ListingModeration {
                is_mature: Some(true),
                is_banned: Some(true),
                category: Some(String::new()),
            })
        );
    }

    #[test]
    fn a_record_without_moderation_takes_the_request() {
        let req = ListingModeration {
            is_mature: Some(false),
            ..Default::default()
        };
        let r = merge_listing(Some(listing_record(None)), Some(&req), 77, 9, &chat());
        assert_eq!(r.moderation, Some(req));
    }

    #[test]
    fn a_missing_request_changes_nothing() {
        let existing = listing_record(Some(ListingModeration {
            is_banned: Some(true),
            ..Default::default()
        }));
        assert_eq!(
            merge_listing(Some(existing.clone()), None, 77, 9, &chat()),
            existing
        );
        let u = new_user_record(3, 9, &[4; 32], UserModeration::default());
        assert_eq!(merge_user(Some(u.clone()), None, true, 77, 9, &[4; 32]), u);
        let fresh = merge_user(None, None, true, 77, 9, &[4; 32]);
        assert_eq!(fresh.id, 77);
        assert_eq!(fresh.moderation, Some(UserModeration::default()));
    }

    #[test]
    fn a_moderator_cannot_grant_moderator_even_on_a_new_record() {
        let req = UserModeration {
            disable_join: Some(true),
            is_moderator: Some(true),
            is_admin: Some(true),
            ..Default::default()
        };
        let r = merge_user(None, Some(&req), false, 77, 9, &[4; 32]);
        assert_eq!(r.id, 77);
        assert_eq!(r.peer_key, vec![4; 32]);
        assert_eq!(
            r.moderation,
            Some(UserModeration {
                disable_join: Some(true),
                ..Default::default()
            })
        );
        let r = merge_user(Some(r), Some(&req), false, 78, 9, &[4; 32]);
        assert!(!is_moderator(Some(&r)));
    }

    #[test]
    fn an_admin_can_grant_and_revoke_moderator() {
        let grant = UserModeration {
            is_moderator: Some(true),
            ..Default::default()
        };
        let r = merge_user(None, Some(&grant), true, 77, 9, &[4; 32]);
        assert!(is_moderator(Some(&r)));
        assert!(!is_admin(Some(&r)));
        let revoke = UserModeration {
            is_moderator: Some(false),
            disable_publish: Some(true),
            ..Default::default()
        };
        let r = merge_user(Some(r), Some(&revoke), true, 78, 9, &[4; 32]);
        assert_eq!(r.id, 77);
        assert_eq!(
            r.moderation,
            Some(UserModeration {
                disable_publish: Some(true),
                is_moderator: Some(false),
                ..Default::default()
            })
        );
    }

    #[test]
    fn permissions_come_from_the_record() {
        assert!(!is_moderator(None));
        let mut r = new_user_record(1, 9, &[4; 32], UserModeration::default());
        r.moderation = None;
        assert!(!is_moderator(Some(&r)));
        let admin = new_user_record(
            1,
            9,
            &[4; 32],
            UserModeration {
                is_admin: Some(true),
                is_moderator: Some(true),
                ..Default::default()
            },
        );
        assert!(is_moderator(Some(&admin)) && is_admin(Some(&admin)));
    }
}
