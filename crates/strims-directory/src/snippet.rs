// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Listing snippets: the deltas between them, the per-reply split of a
//! snippet stream's deltas, the bounds every snippet meets, and the
//! snippets a publisher serves.
//!
//! A delta carries each field whose value changed. An image field set to
//! an image with no source clears the image, so a snippet never holds an
//! empty image: [`merge`] stores one as none and [`diff`] compares them as
//! none. The signature is over the stable encoding, which writes nothing
//! for an empty image either.
//!
//! # Bounds
//!
//! A snippet passes [`check`]: each image encodes to at most
//! [`MAX_IMAGE_BYTES`], the rest of the snippet to at most
//! [`MAX_TEXT_BYTES`], and the whole to at most [`MAX_SNIPPET_BYTES`].
//! Then every delta [`split`] makes from a [`diff`] between two such
//! snippets carries at most one image, and fits one vpn reply
//! ([`MAX_DELTA_BYTES`]).

use std::collections::BTreeMap;

use prost::Message;
use strims_crypto::SignableMessage;
use strims_ppspp::SwarmId;
use strims_proto::network::v1::directory::listing_snippet_delta::{
    ChannelLogoOneof, Tags, TagsOneof, ThumbnailOneof,
};
use strims_proto::network::v1::directory::{
    ListingSnippet, ListingSnippetDelta, ListingSnippetImage,
};
use strims_vpn::MAX_RELAYED_BODY_LEN;

use crate::SnippetError;

/// The longest encoded snippet image.
pub const MAX_IMAGE_BYTES: usize = 32 * 1024;
/// The longest encoded snippet without its images.
pub const MAX_TEXT_BYTES: usize = 8 * 1024;
/// The longest encoded snippet.
pub const MAX_SNIPPET_BYTES: usize = 72 * 1024;

/// The longest certificate header a reply carries: the most a vpn
/// connection accepts by default (`VpnConfig::max_certificate_bytes`).
const MAX_CERTIFICATE_BYTES: usize = 4096;
/// A snippet reply's bytes besides its delta and certificate, rounded up:
/// the Call's id and parent id (11 bytes each at most), its `CALLBACK`
/// method (10), kind (2), and argument key and length (4); the response's
/// delta key and length (4); the certificate header's map entry around
/// the certificate (20).
const REPLY_OVERHEAD: usize = 128;

/// The longest encoded delta: one that fits a vpn RPC reply, which every
/// relay forwards.
pub const MAX_DELTA_BYTES: usize = MAX_RELAYED_BODY_LEN - REPLY_OVERHEAD - MAX_CERTIFICATE_BYTES;

/// The most snippets a publisher holds.
pub const MAX_LOCAL_SNIPPETS: usize = 64;
/// The most snippet subscriptions a publisher serves.
pub const MAX_SNIPPET_SUBSCRIBERS: usize = 64;
/// The most snippet subscriptions a publisher serves for one swarm.
pub const MAX_SUBSCRIBERS_PER_SWARM: usize = 8;

/// The ListingSnippet field numbers of the images.
const THUMBNAIL_FIELD: u32 = 9;
const CHANNEL_LOGO_FIELD: u32 = 10;

/// The delta that turns `a` into `b`.
pub fn diff(a: &ListingSnippet, b: &ListingSnippet) -> ListingSnippetDelta {
    fn changed<T: PartialEq + Clone>(a: &T, b: &T) -> Option<T> {
        (a != b).then(|| b.clone())
    }
    ListingSnippetDelta {
        title: changed(&a.title, &b.title),
        description: changed(&a.description, &b.description),
        category: changed(&a.category, &b.category),
        channel_name: changed(&a.channel_name, &b.channel_name),
        user_count: changed(&a.user_count, &b.user_count),
        live: changed(&a.live, &b.live),
        is_mature: changed(&a.is_mature, &b.is_mature),
        key: changed(&a.key, &b.key),
        signature: changed(&a.signature, &b.signature),
        video_height: changed(&a.video_height, &b.video_height),
        video_width: changed(&a.video_width, &b.video_width),
        theme_color: changed(&a.theme_color, &b.theme_color),
        start_time: changed(&a.start_time, &b.start_time),
        tags_oneof: (a.tags != b.tags).then(|| {
            TagsOneof::Tags(Tags {
                tags: b.tags.clone(),
            })
        }),
        thumbnail_oneof: image_changed(&a.thumbnail, &b.thumbnail).map(ThumbnailOneof::Thumbnail),
        channel_logo_oneof: image_changed(&a.channel_logo, &b.channel_logo)
            .map(ChannelLogoOneof::ChannelLogo),
    }
}

/// `b`, or the empty image for none, if it differs from `a`.
fn image_changed(
    a: &Option<ListingSnippetImage>,
    b: &Option<ListingSnippetImage>,
) -> Option<ListingSnippetImage> {
    (source(a) != source(b)).then(|| source(b).cloned().unwrap_or_default())
}

/// The image, unless it has no source.
fn source(i: &Option<ListingSnippetImage>) -> Option<&ListingSnippetImage> {
    i.as_ref().filter(|i| i.source_oneof.is_some())
}

/// The image of a delta's image field: none for an image with no source.
fn sourced(i: &ListingSnippetImage) -> Option<ListingSnippetImage> {
    i.source_oneof.is_some().then(|| i.clone())
}

/// Applies `d` to `s`.
pub fn merge(s: &mut ListingSnippet, d: &ListingSnippetDelta) {
    fn set<T: Clone>(f: &mut T, v: &Option<T>) {
        if let Some(v) = v {
            f.clone_from(v);
        }
    }
    set(&mut s.title, &d.title);
    set(&mut s.description, &d.description);
    set(&mut s.category, &d.category);
    set(&mut s.channel_name, &d.channel_name);
    set(&mut s.user_count, &d.user_count);
    set(&mut s.live, &d.live);
    set(&mut s.is_mature, &d.is_mature);
    set(&mut s.key, &d.key);
    set(&mut s.signature, &d.signature);
    set(&mut s.video_height, &d.video_height);
    set(&mut s.video_width, &d.video_width);
    set(&mut s.theme_color, &d.theme_color);
    set(&mut s.start_time, &d.start_time);
    if let Some(TagsOneof::Tags(t)) = &d.tags_oneof {
        s.tags.clone_from(&t.tags);
    }
    if let Some(ThumbnailOneof::Thumbnail(i)) = &d.thumbnail_oneof {
        s.thumbnail = sourced(i);
    }
    if let Some(ChannelLogoOneof::ChannelLogo(i)) = &d.channel_logo_oneof {
        s.channel_logo = sourced(i);
    }
}

/// Whether `d` changes nothing.
pub fn is_empty(d: &ListingSnippetDelta) -> bool {
    *d == ListingSnippetDelta::default()
}

/// The deltas a stream sends for `d`, in order: a delta that sets both
/// images goes as the thumbnail alone, then the rest, so that each carries
/// at most one image.
pub fn split(mut d: ListingSnippetDelta) -> Vec<ListingSnippetDelta> {
    let thumbnail = matches!(&d.thumbnail_oneof, Some(ThumbnailOneof::Thumbnail(i)) if i.source_oneof.is_some());
    let logo = matches!(&d.channel_logo_oneof, Some(ChannelLogoOneof::ChannelLogo(i)) if i.source_oneof.is_some());
    if thumbnail && logo {
        let first = ListingSnippetDelta {
            thumbnail_oneof: d.thumbnail_oneof.take(),
            ..ListingSnippetDelta::default()
        };
        vec![first, d]
    } else {
        vec![d]
    }
}

/// Whether `d` fits one vpn reply.
pub fn fits(d: &ListingSnippetDelta) -> bool {
    d.encoded_len() <= MAX_DELTA_BYTES
}

/// Checks `s` against the bounds in the module docs.
pub fn check(s: &ListingSnippet) -> Result<(), SnippetError> {
    let total = s.encoded_len();
    if total > MAX_SNIPPET_BYTES {
        return Err(SnippetError::TooLarge);
    }
    let mut images = 0;
    for (field, image) in [
        (THUMBNAIL_FIELD, &s.thumbnail),
        (CHANNEL_LOGO_FIELD, &s.channel_logo),
    ] {
        if let Some(i) = image {
            if i.encoded_len() > MAX_IMAGE_BYTES {
                return Err(SnippetError::TooLarge);
            }
            images += prost::encoding::message::encoded_len(field, i);
        }
    }
    if total.saturating_sub(images) > MAX_TEXT_BYTES {
        return Err(SnippetError::TooLarge);
    }
    Ok(())
}

/// `s` with its images that have no source cleared.
fn normalized(mut s: ListingSnippet) -> ListingSnippet {
    if source(&s.thumbnail).is_none() {
        s.thumbnail = None;
    }
    if source(&s.channel_logo).is_none() {
        s.channel_logo = None;
    }
    s
}

/// The snippets a publisher serves, by swarm, each with the version it
/// was last changed at. Holds at most [`MAX_LOCAL_SNIPPETS`].
#[derive(Clone, Debug, Default)]
pub struct LocalSnippets {
    snippets: BTreeMap<SwarmId, (ListingSnippet, u64)>,
    version: u64,
}

impl LocalSnippets {
    pub fn new() -> LocalSnippets {
        LocalSnippets::default()
    }

    /// Sets the snippet of swarm `id`, and returns its version: the
    /// current one if the snippet is unchanged, else one greater than any
    /// before. The snippet must pass [`check`] and verify.
    pub fn push(&mut self, id: SwarmId, snippet: ListingSnippet) -> Result<u64, SnippetError> {
        check(&snippet)?;
        snippet
            .verify_message()
            .map_err(|_| SnippetError::InvalidSignature)?;
        let snippet = normalized(snippet);
        if let Some((current, version)) = self.snippets.get_mut(&id) {
            if *current != snippet {
                self.version += 1;
                *current = snippet;
                *version = self.version;
            }
            return Ok(*version);
        }
        if self.snippets.len() >= MAX_LOCAL_SNIPPETS {
            return Err(SnippetError::Full);
        }
        self.version += 1;
        self.snippets.insert(id, (snippet, self.version));
        Ok(self.version)
    }

    /// Removes the snippet of swarm `id`, returning whether there was one.
    pub fn delete(&mut self, id: &SwarmId) -> bool {
        self.snippets.remove(id).is_some()
    }

    /// The snippet of swarm `id` and its version.
    pub fn get(&self, id: &SwarmId) -> Option<(&ListingSnippet, u64)> {
        self.snippets.get(id).map(|(s, v)| (s, *v))
    }

    pub fn len(&self) -> usize {
        self.snippets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snippets.is_empty()
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use std::collections::BTreeMap;

    use strims_crypto::key_from_seed;
    use strims_proto::network::v1::directory::SnippetSubscribeResponse;
    use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
    use strims_proto::rpc::call::Kind;
    use strims_proto::types::{Image, Key};
    use strims_rpc::wire::{CALLBACK, Framing, encode_call_with_headers, encode_headers};

    use super::*;

    fn image(data: usize) -> ListingSnippetImage {
        ListingSnippetImage {
            source_oneof: Some(SourceOneof::Image(Image {
                r#type: 4,
                data: vec![7; data],
                ..Default::default()
            })),
        }
    }

    fn url(u: &str) -> ListingSnippetImage {
        ListingSnippetImage {
            source_oneof: Some(SourceOneof::Url(u.into())),
        }
    }

    fn key() -> Key {
        key_from_seed(&[9; 32])
    }

    fn signed(mut s: ListingSnippet) -> ListingSnippet {
        s.sign_message(&key()).unwrap();
        s
    }

    /// Two snippets signed with one key, differing in most fields.
    fn snippets() -> (ListingSnippet, ListingSnippet) {
        let a = signed(ListingSnippet {
            title: "test a".into(),
            description: "some description".into(),
            tags: vec!["a".into(), "b".into(), "c".into()],
            channel_name: "test channel name".into(),
            is_mature: true,
            video_height: 1080,
            video_width: 1920,
            thumbnail: Some(image(1024)),
            channel_logo: Some(url("https://test.com/a.png")),
            ..Default::default()
        });
        let b = signed(ListingSnippet {
            title: "test b".into(),
            description: "some description".into(),
            category: "test category".into(),
            user_count: 10,
            live: true,
            channel_logo: Some(url("https://test.com/b.png")),
            ..Default::default()
        });
        (a, b)
    }

    #[test]
    fn a_diff_carries_the_changed_fields() {
        let (a, b) = snippets();
        assert_eq!(
            diff(&a, &b),
            ListingSnippetDelta {
                title: Some(b.title.clone()),
                category: Some(b.category.clone()),
                channel_name: Some(String::new()),
                user_count: Some(10),
                live: Some(true),
                is_mature: Some(false),
                video_height: Some(0),
                video_width: Some(0),
                tags_oneof: Some(TagsOneof::Tags(Tags { tags: Vec::new() })),
                thumbnail_oneof: Some(ThumbnailOneof::Thumbnail(ListingSnippetImage::default())),
                channel_logo_oneof: Some(ChannelLogoOneof::ChannelLogo(url(
                    "https://test.com/b.png"
                ))),
                signature: Some(b.signature.clone()),
                ..Default::default()
            }
        );
    }

    #[test]
    fn the_empty_delta_is_empty() {
        assert!(is_empty(&ListingSnippetDelta::default()));
        assert!(!is_empty(&ListingSnippetDelta {
            live: Some(false),
            ..Default::default()
        }));
    }

    #[test]
    fn equal_snippets_diff_to_nothing() {
        let (a, _) = snippets();
        assert!(is_empty(&diff(&a, &a)));
    }

    #[test]
    fn merging_a_diff_gives_its_target() {
        let (a, b) = snippets();
        let mut c = a.clone();
        merge(&mut c, &diff(&a, &b));
        assert_eq!(c, b);
        c.verify_message().unwrap();
        let mut d = b.clone();
        merge(&mut d, &diff(&b, &a));
        assert_eq!(d, a);
    }

    #[test]
    fn merging_into_the_empty_snippet_copies() {
        let (_, b) = snippets();
        let mut c = ListingSnippet::default();
        let d = diff(&c, &b);
        merge(&mut c, &d);
        assert_eq!(c, b);
    }

    #[test]
    fn an_image_without_a_source_clears_the_image() {
        let (a, _) = snippets();
        let mut c = a.clone();
        merge(
            &mut c,
            &ListingSnippetDelta {
                thumbnail_oneof: Some(ThumbnailOneof::Thumbnail(ListingSnippetImage::default())),
                ..Default::default()
            },
        );
        assert_eq!(c.thumbnail, None);
        // the empty image is none for diffs, and for the signature
        let mut e = a.clone();
        e.thumbnail = Some(ListingSnippetImage::default());
        assert!(is_empty(&diff(&c, &e)));
        let mut cleared = a.clone();
        cleared.thumbnail = None;
        cleared.sign_message(&key()).unwrap();
        e.signature.clone_from(&cleared.signature);
        e.verify_message().unwrap();
    }

    #[test]
    fn a_delta_with_both_images_sends_the_thumbnail_first() {
        let (a, _) = snippets();
        let full = diff(&ListingSnippet::default(), &a);
        let parts = split(full.clone());
        assert_eq!(parts.len(), 2);
        assert_eq!(
            parts[0],
            ListingSnippetDelta {
                thumbnail_oneof: full.thumbnail_oneof.clone(),
                ..Default::default()
            }
        );
        assert_eq!(parts[1].thumbnail_oneof, None);
        assert_eq!(parts[1].channel_logo_oneof, full.channel_logo_oneof);
        let mut s = ListingSnippet::default();
        for p in &parts {
            merge(&mut s, p);
        }
        assert_eq!(s, a);

        // clearing one image while setting the other isn't split
        let mut b = a.clone();
        b.thumbnail = None;
        b.channel_logo = Some(image(10));
        assert_eq!(split(diff(&a, &b)).len(), 1);
    }

    /// The largest snippets within the bounds: the most text (as many
    /// empty tags as fit, the field that grows most in a delta) and two
    /// images of the largest size.
    fn largest(text_seed: u8) -> ListingSnippet {
        let image_data = MAX_IMAGE_BYTES - 16;
        let mut s = ListingSnippet {
            title: format!("{text_seed}"),
            thumbnail: Some(image(image_data)),
            channel_logo: Some(url(&"u".repeat(MAX_IMAGE_BYTES - 8))),
            ..Default::default()
        };
        s.sign_message(&key_from_seed(&[text_seed; 32])).unwrap();
        while check(&s).is_ok() {
            s.tags.push(String::new());
        }
        s.tags.pop();
        s.sign_message(&key_from_seed(&[text_seed; 32])).unwrap();
        check(&s).unwrap();
        s
    }

    #[test]
    fn every_delta_between_snippets_within_bounds_fits_one_reply() {
        let a = largest(1);
        let b = largest(2);
        assert!(a.thumbnail.as_ref().unwrap().encoded_len() > MAX_IMAGE_BYTES - 16);
        for (from, to) in [
            (ListingSnippet::default(), a.clone()),
            (a.clone(), b.clone()),
            (b.clone(), ListingSnippet::default()),
        ] {
            for d in split(diff(&from, &to)) {
                assert!(fits(&d), "{} > {MAX_DELTA_BYTES}", d.encoded_len());
            }
        }
    }

    #[test]
    fn the_largest_delta_fits_a_vpn_reply() {
        let config = strims_rpc::vpn::VpnConfig::default();
        assert!(config.max_certificate_bytes <= MAX_CERTIFICATE_BYTES);
        assert_eq!(config.rpc.max_send_bytes, MAX_RELAYED_BODY_LEN);

        let mut d = ListingSnippetDelta {
            title: Some(String::new()),
            ..Default::default()
        };
        let pad = MAX_DELTA_BYTES - d.encoded_len();
        d.title = Some("t".repeat(pad));
        while d.encoded_len() > MAX_DELTA_BYTES {
            d.title.as_mut().unwrap().pop();
        }
        assert!(d.encoded_len() > MAX_DELTA_BYTES - 4);
        let response = SnippetSubscribeResponse {
            snippet_delta: Some(d),
        };
        let headers = encode_headers(&BTreeMap::from([(
            strims_rpc::vpn::CERTIFICATE_HEADER.to_owned(),
            vec![0xff; MAX_CERTIFICATE_BYTES],
        )]));
        let mut out = Vec::new();
        let n = encode_call_with_headers(
            &mut out,
            Framing::Message,
            u64::MAX,
            u64::MAX,
            CALLBACK,
            Kind::CallKindError,
            &response,
            &headers,
        );
        assert_eq!(n, out.len());
        assert!(n <= MAX_RELAYED_BODY_LEN, "{n} > {MAX_RELAYED_BODY_LEN}");
    }

    #[test]
    fn snippets_over_a_bound_are_refused() {
        let s = largest(3);
        check(&s).unwrap();
        let mut big_image = s.clone();
        big_image.thumbnail = Some(image(MAX_IMAGE_BYTES));
        assert_eq!(check(&big_image), Err(SnippetError::TooLarge));
        let mut big_text = ListingSnippet {
            description: "d".repeat(MAX_TEXT_BYTES),
            ..Default::default()
        };
        assert_eq!(check(&big_text), Err(SnippetError::TooLarge));
        big_text.description.truncate(MAX_TEXT_BYTES - 8);
        assert_eq!(check(&big_text), Ok(()));
        let mut big = s;
        big.description = "d".repeat(64);
        assert_eq!(check(&big), Err(SnippetError::TooLarge));
    }

    #[test]
    fn local_snippets_version_their_changes() {
        let (a, b) = snippets();
        let mut l = LocalSnippets::new();
        let id = SwarmId::new(&[1; 32]);
        let v1 = l.push(id.clone(), a.clone()).unwrap();
        assert_eq!(l.push(id.clone(), a.clone()).unwrap(), v1);
        let v2 = l.push(id.clone(), b.clone()).unwrap();
        assert!(v2 > v1);
        assert_eq!(l.get(&id), Some((&b, v2)));
        let other = SwarmId::new(&[2; 32]);
        assert!(l.push(other.clone(), a.clone()).unwrap() > v2);
        assert_eq!(l.len(), 2);
        assert!(l.delete(&other));
        assert!(!l.delete(&other));
        assert_eq!(l.get(&other), None);
    }

    #[test]
    fn local_snippets_refuse_unsigned_oversized_and_too_many() {
        let (a, _) = snippets();
        let mut l = LocalSnippets::new();
        let mut forged = a.clone();
        forged.title = "forged".into();
        assert_eq!(
            l.push(SwarmId::new(&[1]), forged),
            Err(SnippetError::InvalidSignature)
        );
        let mut big = a.clone();
        big.thumbnail = Some(image(MAX_IMAGE_BYTES));
        assert_eq!(l.push(SwarmId::new(&[1]), big), Err(SnippetError::TooLarge));
        for i in 0..MAX_LOCAL_SNIPPETS {
            l.push(SwarmId::new(&i.to_be_bytes()), a.clone()).unwrap();
        }
        assert_eq!(
            l.push(SwarmId::new(&[1]), a.clone()),
            Err(SnippetError::Full)
        );
        // an existing swarm still updates
        l.push(SwarmId::new(&0usize.to_be_bytes()), a).unwrap();
        assert!(!l.is_empty());
    }

    #[test]
    fn pushed_empty_images_are_stored_as_none() {
        let (a, _) = snippets();
        let mut e = a.clone();
        e.thumbnail = Some(ListingSnippetImage::default());
        let mut cleared = a.clone();
        cleared.thumbnail = None;
        cleared.sign_message(&key()).unwrap();
        e.signature.clone_from(&cleared.signature);
        let mut l = LocalSnippets::new();
        l.push(SwarmId::new(&[1]), e).unwrap();
        assert_eq!(l.get(&SwarmId::new(&[1])).unwrap().0, &cleared);
    }
}
