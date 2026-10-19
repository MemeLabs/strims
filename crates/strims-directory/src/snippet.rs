// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Listing snippets: the signed form a media listing's snippet travels in,
//! the bounds every snippet meets, the responses of a snippet stream, and
//! the snippets a publisher serves.
//!
//! # Signed snippets
//!
//! A media listing's snippet travels as a `SignedListingSnippet`: the
//! encoding of a `ListingSnippetBody` and its signature, made in
//! [`Domain::DirectoryListingSnippet`](strims_crypto::Domain) by the
//! listing's media swarm key, the key whose public half is the swarm id.
//! The body holds the snippet's content with its images unset, the
//! [`digest`] of each image's encoded `ListingSnippetImage`, and the unix
//! time in nanoseconds it was signed at. The images travel beside it,
//! encoded. Whoever holds or forwards a signed snippet keeps its envelope
//! and its images' bytes as they arrived ([`SignedSnippet`]), so fields of
//! the body or an image this node doesn't know survive.
//!
//! An image without a source is none: it has no digest. A body whose
//! content holds an image is malformed, as is a digest that isn't 32 bytes
//! or a named image that doesn't decode.
//!
//! # Streams
//!
//! A snippet stream brings its subscriber from the last snippet it sent to
//! the current one ([`Feed`]): each image of the current snippet that the
//! last one didn't name, then the signed body, with the last of those
//! images in the body's response. So every response carries at most one
//! image, and a subscriber needs to keep at most two images that no
//! snippet it holds names.
//!
//! # Bounds
//!
//! A snippet passes [`check`]: each image encodes to at most
//! [`MAX_IMAGE_BYTES`], the rest of the snippet to at most
//! [`MAX_TEXT_BYTES`], and the whole to at most [`MAX_SNIPPET_BYTES`]. A
//! signed snippet's body encodes to at most [`MAX_BODY_BYTES`], and each
//! of its images to at most [`MAX_IMAGE_BYTES`] as received. Then every
//! stream response fits one vpn reply ([`MAX_RESPONSE_BYTES`]).

use std::collections::BTreeMap;

use prost::Message;
use sha2::{Digest as _, Sha256};
use strims_base::time::Time;
use strims_crypto::{Error as CryptoError, Signer, Verified};
use strims_ppspp::SwarmId;
use strims_proto::network::v1::directory::{
    ListingSnippet, ListingSnippetBody, ListingSnippetImage, SignedListingSnippet,
    SnippetSubscribeResponse,
};
use strims_vpn::MAX_RELAYED_BODY_LEN;

use crate::SnippetError;

/// The longest encoded snippet image.
pub const MAX_IMAGE_BYTES: usize = 32 * 1024;
/// The longest encoded snippet without its images.
pub const MAX_TEXT_BYTES: usize = 8 * 1024;
/// The longest encoded snippet.
pub const MAX_SNIPPET_BYTES: usize = 72 * 1024;
/// The longest encoded snippet body: a snippet's text with its digests and
/// timestamp, and room for fields this node doesn't know.
pub const MAX_BODY_BYTES: usize = MAX_TEXT_BYTES + 1024;
/// The longest encoded signed snippet: a body of [`MAX_BODY_BYTES`] and
/// its signature, with their keys and lengths.
pub const MAX_SIGNED_BYTES: usize = MAX_BODY_BYTES + 128;

/// The longest certificate header a reply carries: the most a vpn
/// connection accepts by default (`VpnConfig::max_certificate_bytes`).
const MAX_CERTIFICATE_BYTES: usize = 4096;
/// A snippet reply's bytes besides its response and certificate, rounded
/// up: the Call's id and parent id (11 bytes each at most), its `CALLBACK`
/// method (10), kind (2), and argument key and length (4); the certificate
/// header's map entry around the certificate (20).
const REPLY_OVERHEAD: usize = 128;

/// The longest encoded stream response: one that fits a vpn RPC reply,
/// which every relay forwards. A signed snippet of [`MAX_SIGNED_BYTES`]
/// with an image of [`MAX_IMAGE_BYTES`] fits.
pub const MAX_RESPONSE_BYTES: usize = MAX_RELAYED_BODY_LEN - REPLY_OVERHEAD - MAX_CERTIFICATE_BYTES;

/// The most snippets a publisher holds.
pub const MAX_LOCAL_SNIPPETS: usize = 64;
/// The most snippet subscriptions a publisher serves.
pub const MAX_SNIPPET_SUBSCRIBERS: usize = 64;
/// The most snippet subscriptions a publisher serves for one swarm.
pub const MAX_SUBSCRIBERS_PER_SWARM: usize = 8;

/// The ListingSnippet field numbers of the images.
const THUMBNAIL_FIELD: u32 = 9;
const CHANNEL_LOGO_FIELD: u32 = 10;

/// The SHA-256 of an encoded image.
pub type Digest = [u8; 32];

/// An image's encoding and its digest.
pub type EncodedImage = (Digest, Vec<u8>);

/// The digest of an encoded image.
pub fn digest(image: &[u8]) -> Digest {
    Sha256::digest(image).into()
}

/// The encoding of the image `digest` names among `images`.
pub fn image_in<'a>(images: &'a [EncodedImage], digest: &Digest) -> Option<&'a [u8]> {
    images
        .iter()
        .find(|(d, _)| d == digest)
        .map(|(_, b)| b.as_slice())
}

/// The body of `snippet` at `timestamp`, and the encodings of the images
/// it names, each once.
pub(crate) fn seal(
    snippet: &ListingSnippet,
    timestamp: i64,
) -> (ListingSnippetBody, Vec<EncodedImage>) {
    let mut images: Vec<EncodedImage> = Vec::new();
    let mut name = |image: &Option<ListingSnippetImage>| -> Vec<u8> {
        let Some(i) = image.as_ref().filter(|i| i.source_oneof.is_some()) else {
            return Vec::new();
        };
        let bytes = i.encode_to_vec();
        let d = digest(&bytes);
        if image_in(&images, &d).is_none() {
            images.push((d, bytes));
        }
        d.to_vec()
    };
    let thumbnail_digest = name(&snippet.thumbnail);
    let channel_logo_digest = name(&snippet.channel_logo);
    let mut content = snippet.clone();
    content.thumbnail = None;
    content.channel_logo = None;
    let body = ListingSnippetBody {
        snippet: Some(content),
        thumbnail_digest,
        channel_logo_digest,
        timestamp,
    };
    (body, images)
}

/// The well-formed digests `body` names, each once.
fn digests(body: &ListingSnippetBody) -> Vec<Digest> {
    let mut out = Vec::with_capacity(2);
    for d in [&body.thumbnail_digest, &body.channel_logo_digest] {
        if let Ok(d) = Digest::try_from(d.as_slice())
            && !out.contains(&d)
        {
            out.push(d);
        }
    }
    out
}

/// The snippet `body` states, with the images it names inline, each taken
/// from `image` by digest.
///
/// # Errors
///
/// [`SnippetError::Malformed`] when the body's content holds an image, a
/// digest isn't 32 bytes or an image doesn't decode, and
/// [`SnippetError::MissingImage`] when `image` lacks a named image.
pub fn resolve<'a>(
    body: &ListingSnippetBody,
    image: impl Fn(&Digest) -> Option<&'a [u8]>,
) -> Result<ListingSnippet, SnippetError> {
    let mut s = body.snippet.clone().unwrap_or_default();
    if s.thumbnail.is_some() || s.channel_logo.is_some() {
        return Err(SnippetError::Malformed);
    }
    s.thumbnail = named(&body.thumbnail_digest, &image)?;
    s.channel_logo = named(&body.channel_logo_digest, &image)?;
    Ok(s)
}

/// The image `digest` names: none for the empty digest.
fn named<'a>(
    digest: &[u8],
    image: &impl Fn(&Digest) -> Option<&'a [u8]>,
) -> Result<Option<ListingSnippetImage>, SnippetError> {
    if digest.is_empty() {
        return Ok(None);
    }
    let d = Digest::try_from(digest).map_err(|_| SnippetError::Malformed)?;
    let bytes = image(&d).ok_or(SnippetError::MissingImage)?;
    ListingSnippetImage::decode(bytes)
        .map(Some)
        .map_err(|_| SnippetError::Malformed)
}

/// The snippet `envelope` states, without verifying its signature, with
/// the images it names taken from `images` by digest. Errors as
/// [`resolve`], and [`SnippetError::Malformed`] for a body that doesn't
/// decode.
pub fn decode(
    envelope: &SignedListingSnippet,
    images: &[Vec<u8>],
) -> Result<ListingSnippet, SnippetError> {
    let body = ListingSnippetBody::decode(envelope.body.as_slice())
        .map_err(|_| SnippetError::Malformed)?;
    resolve(&body, |d| {
        images.iter().find(|i| digest(i) == *d).map(Vec::as_slice)
    })
}

/// Whether two bodies state the same snippet, whenever they were signed.
pub(crate) fn same_content(a: &ListingSnippetBody, b: &ListingSnippetBody) -> bool {
    a.snippet == b.snippet
        && a.thumbnail_digest == b.thumbnail_digest
        && a.channel_logo_digest == b.channel_logo_digest
}

/// A signed snippet and the encodings of the images its body names, as
/// signed or received; see the [module docs](self).
#[derive(Clone, Debug, PartialEq)]
pub struct SignedSnippet {
    signed: Verified<ListingSnippetBody>,
    /// By digest, each once: at most two.
    images: Vec<EncodedImage>,
}

impl SignedSnippet {
    /// Signs `snippet` at `now` with `signer`, its media swarm's key.
    pub fn sign(signer: &Signer, snippet: &ListingSnippet, now: Time) -> SignedSnippet {
        let (body, images) = seal(snippet, now.unix_nanos());
        SignedSnippet {
            signed: Verified::sign(signer, body),
            images,
        }
    }

    /// Checks that the key of swarm `swarm_id` signed `envelope`, takes the
    /// images its body names from `image` by digest, and checks the
    /// snippet against the bounds in the module docs.
    ///
    /// # Errors
    ///
    /// [`SnippetError::InvalidSignature`], [`SnippetError::TooLarge`], and
    /// the errors of [`resolve`].
    pub fn verify<'a>(
        envelope: SignedListingSnippet,
        swarm_id: &[u8],
        image: impl Fn(&Digest) -> Option<&'a [u8]>,
    ) -> Result<SignedSnippet, SnippetError> {
        if envelope.body.len() > MAX_BODY_BYTES {
            return Err(SnippetError::TooLarge);
        }
        let signed = Verified::<ListingSnippetBody>::verify(envelope, swarm_id).map_err(|e| {
            if e == CryptoError::MalformedBody {
                SnippetError::Malformed
            } else {
                SnippetError::InvalidSignature
            }
        })?;
        let mut images = Vec::with_capacity(2);
        for d in digests(signed.body()) {
            if let Some(b) = image(&d) {
                if b.len() > MAX_IMAGE_BYTES {
                    return Err(SnippetError::TooLarge);
                }
                images.push((d, b.to_vec()));
            }
        }
        check(&resolve(signed.body(), |d| image_in(&images, d))?)?;
        Ok(SignedSnippet { signed, images })
    }

    /// The envelope, as signed or received.
    pub fn envelope(&self) -> &SignedListingSnippet {
        self.signed.envelope()
    }

    pub fn body(&self) -> &ListingSnippetBody {
        self.signed.body()
    }

    /// When it was signed, in unix nanoseconds.
    pub fn timestamp(&self) -> i64 {
        self.signed.timestamp
    }

    /// The encodings of the images the body names, by digest.
    pub fn images(&self) -> &[EncodedImage] {
        &self.images
    }

    /// The snippet, with its images inline.
    pub fn snippet(&self) -> ListingSnippet {
        // invariant: signing and verifying resolve the body against these
        // images, so this only fails for a snippet built otherwise.
        resolve(self.body(), |d| image_in(&self.images, d)).unwrap_or_default()
    }

    pub fn into_parts(self) -> (SignedListingSnippet, Vec<EncodedImage>) {
        (self.signed.into_envelope(), self.images)
    }
}

/// What a snippet stream has sent its subscriber; see the
/// [module docs](self#streams).
#[derive(Clone, Debug, Default)]
pub struct Feed {
    /// The digests the last snippet sent names.
    named: Vec<Digest>,
}

impl Feed {
    /// The responses that bring the subscriber to `s`, in order: each
    /// image of `s` that the last snippet sent didn't name, then `s`, with
    /// the last such image in its response.
    pub fn responses(&mut self, s: &SignedSnippet) -> Vec<SnippetSubscribeResponse> {
        let mut images: Vec<&[u8]> = s
            .images
            .iter()
            .filter(|(d, _)| !self.named.contains(d))
            .map(|(_, b)| b.as_slice())
            .collect();
        let last = images.pop();
        let mut out: Vec<SnippetSubscribeResponse> = images
            .into_iter()
            .map(|b| SnippetSubscribeResponse {
                snippet: None,
                image: b.to_vec(),
            })
            .collect();
        out.push(SnippetSubscribeResponse {
            snippet: Some(s.envelope().clone()),
            image: last.map(<[u8]>::to_vec).unwrap_or_default(),
        });
        self.named = s.images.iter().map(|(d, _)| *d).collect();
        out
    }
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

/// The snippets a publisher serves, by swarm, each with the version it
/// was last changed at. Holds at most [`MAX_LOCAL_SNIPPETS`].
#[derive(Clone, Debug, Default)]
pub struct LocalSnippets {
    snippets: BTreeMap<SwarmId, (SignedSnippet, u64)>,
    version: u64,
}

impl LocalSnippets {
    pub fn new() -> LocalSnippets {
        LocalSnippets::default()
    }

    /// Sets the snippet of swarm `id`, and returns its version: the
    /// current one if the snippet states what the current one does, else
    /// one greater than any before. The snippet must verify against the
    /// swarm, as [`SignedSnippet::verify`] checks.
    pub fn push(&mut self, id: SwarmId, snippet: SignedSnippet) -> Result<u64, SnippetError> {
        let (envelope, images) = snippet.into_parts();
        let snippet = SignedSnippet::verify(envelope, id.as_bytes(), |d| image_in(&images, d))?;
        if let Some((current, version)) = self.snippets.get_mut(&id) {
            if !same_content(current.body(), snippet.body()) {
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
    pub fn get(&self, id: &SwarmId) -> Option<(&SignedSnippet, u64)> {
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

    use strims_base::time::Duration;
    use strims_crypto::{Domain, key_from_seed, sign};
    use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
    use strims_proto::rpc::call::Kind;
    use strims_proto::types::{Image, Key};
    use strims_rpc::wire::{CALLBACK, Framing, encode_call_with_headers, encode_headers};

    use super::*;

    const T0: Time = Time::from_unix_millis(1_700_000_000_000);

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

    fn signer() -> Signer {
        Signer::new(&key()).unwrap()
    }

    fn swarm() -> SwarmId {
        SwarmId::new(&key().public)
    }

    fn signed(s: &ListingSnippet) -> SignedSnippet {
        SignedSnippet::sign(&signer(), s, T0)
    }

    /// Verifies `s`'s envelope against the swarm with its own images.
    fn reverify(s: &SignedSnippet) -> Result<SignedSnippet, SnippetError> {
        SignedSnippet::verify(s.envelope().clone(), swarm().as_bytes(), |d| {
            image_in(s.images(), d)
        })
    }

    /// Two snippets, differing in most fields.
    fn snippets() -> (ListingSnippet, ListingSnippet) {
        let a = ListingSnippet {
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
        };
        let b = ListingSnippet {
            title: "test b".into(),
            description: "some description".into(),
            category: "test category".into(),
            user_count: 10,
            live: true,
            channel_logo: Some(url("https://test.com/b.png")),
            ..Default::default()
        };
        (a, b)
    }

    #[test]
    fn a_signed_snippet_verifies_against_its_swarm() {
        let (a, _) = snippets();
        let s = signed(&a);
        assert_eq!(s.snippet(), a);
        assert_eq!(s.timestamp(), T0.unix_nanos());
        let v = reverify(&s).unwrap();
        assert_eq!(v, s);
        assert_eq!(v.snippet(), a);
        // the body holds the content without its images
        let body = s.body();
        assert_eq!(body.snippet.as_ref().unwrap().thumbnail, None);
        assert_eq!(body.snippet.as_ref().unwrap().channel_logo, None);
        assert_eq!(body.snippet.as_ref().unwrap().title, a.title);
    }

    #[test]
    fn a_snippet_signed_by_another_key_is_rejected() {
        let (a, _) = snippets();
        let other = Signer::new(&key_from_seed(&[8; 32])).unwrap();
        let s = SignedSnippet::sign(&other, &a, T0);
        assert_eq!(reverify(&s), Err(SnippetError::InvalidSignature));
        let mut tampered = signed(&a).envelope().clone();
        tampered.body.push(0);
        assert_eq!(
            SignedSnippet::verify(tampered, swarm().as_bytes(), |_| None),
            Err(SnippetError::InvalidSignature)
        );
    }

    #[test]
    fn a_snippet_signature_verifies_in_no_other_domain() {
        let (a, _) = snippets();
        let s = signed(&a);
        let env = s.envelope();
        let public = key().public;
        assert!(sign::verify(
            Domain::DirectoryListingSnippet,
            &public,
            &[&env.body],
            &env.signature
        ));
        for d in Domain::ALL
            .into_iter()
            .filter(|&d| d != Domain::DirectoryListingSnippet)
        {
            assert!(
                !sign::verify(d, &public, &[&env.body], &env.signature),
                "{d:?}"
            );
        }
    }

    #[test]
    fn snippet_images_resolve_by_digest() {
        let (a, _) = snippets();
        let s = signed(&a);
        let thumbnail = a.thumbnail.as_ref().unwrap().encode_to_vec();
        let logo = a.channel_logo.as_ref().unwrap().encode_to_vec();
        assert_eq!(s.body().thumbnail_digest, digest(&thumbnail));
        assert_eq!(s.body().channel_logo_digest, digest(&logo));
        assert_eq!(
            s.images(),
            [
                (digest(&thumbnail), thumbnail.clone()),
                (digest(&logo), logo.clone())
            ]
        );
        // images come in any order, among others
        let images = vec![b"other".to_vec(), logo.clone(), thumbnail.clone()];
        assert_eq!(decode(s.envelope(), &images), Ok(a.clone()));
        let by_digest: BTreeMap<Digest, Vec<u8>> =
            images.iter().map(|i| (digest(i), i.clone())).collect();
        let v = SignedSnippet::verify(s.envelope().clone(), swarm().as_bytes(), |d| {
            by_digest.get(d).map(Vec::as_slice)
        })
        .unwrap();
        assert_eq!(v.snippet(), a);
        // a missing image fails
        assert_eq!(
            decode(s.envelope(), &[thumbnail]),
            Err(SnippetError::MissingImage)
        );
        assert_eq!(
            SignedSnippet::verify(s.envelope().clone(), swarm().as_bytes(), |d| {
                (*d == digest(&logo)).then_some(logo.as_slice())
            }),
            Err(SnippetError::MissingImage)
        );
    }

    #[test]
    fn one_image_in_both_places_travels_once() {
        let s = signed(&ListingSnippet {
            thumbnail: Some(image(10)),
            channel_logo: Some(image(10)),
            ..Default::default()
        });
        assert_eq!(s.images().len(), 1);
        assert_eq!(s.body().thumbnail_digest, s.body().channel_logo_digest);
        assert_eq!(
            reverify(&s).unwrap().snippet().channel_logo,
            Some(image(10))
        );
    }

    #[test]
    fn an_image_without_a_source_is_none() {
        let (mut a, _) = snippets();
        a.thumbnail = Some(ListingSnippetImage::default());
        let s = signed(&a);
        assert!(s.body().thumbnail_digest.is_empty());
        assert_eq!(s.images().len(), 1);
        assert_eq!(s.snippet().thumbnail, None);
        a.thumbnail = None;
        assert!(same_content(s.body(), signed(&a).body()));
    }

    /// Signs `body` as is.
    fn sign_body(body: ListingSnippetBody) -> SignedListingSnippet {
        Verified::sign(&signer(), body).into_envelope()
    }

    #[test]
    fn a_body_with_inline_images_is_rejected() {
        let (a, _) = snippets();
        let (body, images) = seal(&a, 1);
        let mut inline = body.clone();
        inline.snippet.as_mut().unwrap().channel_logo = a.channel_logo.clone();
        let env = sign_body(inline);
        assert_eq!(
            SignedSnippet::verify(env.clone(), swarm().as_bytes(), |d| image_in(&images, d)),
            Err(SnippetError::Malformed)
        );
        assert_eq!(decode(&env, &[]), Err(SnippetError::Malformed));
    }

    #[test]
    fn digests_that_are_not_32_bytes_are_rejected() {
        let (a, _) = snippets();
        let (body, images) = seal(&a, 1);
        for len in [1, 31, 33] {
            let mut short = body.clone();
            short.thumbnail_digest.resize(len, 0);
            let env = sign_body(short);
            assert_eq!(
                SignedSnippet::verify(env, swarm().as_bytes(), |d| image_in(&images, d)),
                Err(SnippetError::Malformed),
                "{len}"
            );
        }
        let env = sign_body(body);
        assert!(SignedSnippet::verify(env, swarm().as_bytes(), |d| image_in(&images, d)).is_ok());
    }

    #[test]
    fn a_body_or_image_that_does_not_decode_is_malformed() {
        let mut env = sign_body(ListingSnippetBody::default());
        env.body = vec![0xff];
        env.signature = signer()
            .sign(Domain::DirectoryListingSnippet, &[&env.body])
            .to_vec();
        assert_eq!(
            SignedSnippet::verify(env.clone(), swarm().as_bytes(), |_| None),
            Err(SnippetError::Malformed)
        );
        assert_eq!(decode(&env, &[]), Err(SnippetError::Malformed));
        let garbage = vec![0xff; 4];
        let env = sign_body(ListingSnippetBody {
            thumbnail_digest: digest(&garbage).to_vec(),
            ..Default::default()
        });
        assert_eq!(decode(&env, &[garbage]), Err(SnippetError::Malformed));
    }

    #[test]
    fn the_feed_sends_the_images_a_subscriber_lacks_before_the_body() {
        let (a, b) = snippets();
        let sa = signed(&a);
        let mut feed = Feed::default();
        let r = feed.responses(&sa);
        assert_eq!(r.len(), 2);
        assert_eq!(
            (r[0].snippet.as_ref(), &r[0].image),
            (None, &sa.images()[0].1)
        );
        assert_eq!(
            (r[1].snippet.as_ref(), &r[1].image),
            (Some(sa.envelope()), &sa.images()[1].1)
        );
        // the same images go once
        let later = SignedSnippet::sign(&signer(), &a, T0 + Duration::SECOND);
        let r = feed.responses(&later);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].snippet.as_ref(), Some(later.envelope()));
        assert!(r[0].image.is_empty());
        // a changed image goes with the body
        let sb = signed(&b);
        let r = feed.responses(&sb);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].image, sb.images()[0].1);
        // and so does a snippet without images
        let none = signed(&ListingSnippet::default());
        let r = feed.responses(&none);
        assert_eq!(r.len(), 1);
        assert!(r[0].image.is_empty());
        // a fresh feed sends every image
        assert_eq!(Feed::default().responses(&sa).len(), 2);
    }

    /// The largest snippets within the bounds: the most text (as many
    /// empty tags as fit) and two images of the largest size.
    fn largest(text_seed: u8) -> ListingSnippet {
        let image_data = MAX_IMAGE_BYTES - 16;
        let mut s = ListingSnippet {
            title: format!("{text_seed}"),
            thumbnail: Some(image(image_data)),
            channel_logo: Some(url(&"u".repeat(MAX_IMAGE_BYTES - 8))),
            start_time: i64::MIN,
            ..Default::default()
        };
        while check(&s).is_ok() {
            s.tags.push(String::new());
        }
        s.tags.pop();
        check(&s).unwrap();
        s
    }

    #[test]
    fn every_response_of_a_snippet_within_bounds_fits_one_reply() {
        let a = largest(1);
        assert!(a.thumbnail.as_ref().unwrap().encoded_len() > MAX_IMAGE_BYTES - 16);
        let s = SignedSnippet::sign(&signer(), &a, Time::from_unix_nanos(i64::MIN));
        let s = reverify(&s).unwrap();
        assert!(s.envelope().body.len() <= MAX_BODY_BYTES);
        assert!(s.envelope().encoded_len() <= MAX_SIGNED_BYTES);
        for r in Feed::default().responses(&s) {
            assert!(
                r.encoded_len() <= MAX_RESPONSE_BYTES,
                "{} > {MAX_RESPONSE_BYTES}",
                r.encoded_len()
            );
        }
        // the bounds' largest response fits too
        let largest = SnippetSubscribeResponse {
            snippet: Some(SignedListingSnippet {
                body: vec![0; MAX_BODY_BYTES],
                signature: vec![0; 64],
            }),
            image: vec![0; MAX_IMAGE_BYTES],
        };
        assert!(largest.snippet.as_ref().unwrap().encoded_len() <= MAX_SIGNED_BYTES);
        assert!(largest.encoded_len() <= MAX_RESPONSE_BYTES);
    }

    #[test]
    fn the_largest_response_fits_a_vpn_reply() {
        let config = strims_rpc::vpn::VpnConfig::default();
        assert!(config.max_certificate_bytes <= MAX_CERTIFICATE_BYTES);
        assert_eq!(config.rpc.max_send_bytes, MAX_RELAYED_BODY_LEN);

        let mut response = SnippetSubscribeResponse::default();
        let pad = MAX_RESPONSE_BYTES - 4;
        response.image = vec![0xff; pad];
        while response.encoded_len() > MAX_RESPONSE_BYTES {
            response.image.pop();
        }
        assert!(response.encoded_len() > MAX_RESPONSE_BYTES - 4);
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
    fn signed_snippets_over_a_bound_are_refused() {
        let mut big_image = largest(4);
        big_image.thumbnail = Some(image(MAX_IMAGE_BYTES));
        assert_eq!(reverify(&signed(&big_image)), Err(SnippetError::TooLarge));
        // an image or body over its bound as received, whatever it decodes to
        let s = signed(&ListingSnippet {
            thumbnail: Some(image(10)),
            ..Default::default()
        });
        let mut padded = s.images()[0].1.clone();
        padded.extend(std::iter::repeat_n(0, MAX_IMAGE_BYTES));
        let env = sign_body(ListingSnippetBody {
            thumbnail_digest: digest(&padded).to_vec(),
            ..Default::default()
        });
        assert_eq!(
            SignedSnippet::verify(env, swarm().as_bytes(), |_| Some(&padded)),
            Err(SnippetError::TooLarge)
        );
        let mut env = s.envelope().clone();
        env.body.resize(MAX_BODY_BYTES + 1, 0);
        assert_eq!(
            SignedSnippet::verify(env, swarm().as_bytes(), |_| None),
            Err(SnippetError::TooLarge)
        );
    }

    #[test]
    fn local_snippets_version_their_changes() {
        let (a, b) = snippets();
        let mut l = LocalSnippets::new();
        let id = swarm();
        let v1 = l.push(id.clone(), signed(&a)).unwrap();
        // the same content signed later keeps its version
        let later = SignedSnippet::sign(&signer(), &a, T0 + Duration::SECOND);
        assert_eq!(l.push(id.clone(), later).unwrap(), v1);
        assert_eq!(l.get(&id).unwrap().0.timestamp(), T0.unix_nanos());
        let v2 = l.push(id.clone(), signed(&b)).unwrap();
        assert!(v2 > v1);
        assert_eq!(l.get(&id), Some((&signed(&b), v2)));
        let other_key = key_from_seed(&[2; 32]);
        let other = SwarmId::new(&other_key.public);
        let theirs = SignedSnippet::sign(&Signer::new(&other_key).unwrap(), &a, T0);
        assert!(l.push(other.clone(), theirs).unwrap() > v2);
        assert_eq!(l.len(), 2);
        assert!(l.delete(&other));
        assert!(!l.delete(&other));
        assert_eq!(l.get(&other), None);
    }

    #[test]
    fn local_snippets_refuse_unsigned_oversized_and_too_many() {
        let (a, _) = snippets();
        let mut l = LocalSnippets::new();
        // signed by a key other than the swarm's
        assert_eq!(
            l.push(SwarmId::new(&[1; 32]), signed(&a)),
            Err(SnippetError::InvalidSignature)
        );
        let mut big = a.clone();
        big.thumbnail = Some(image(MAX_IMAGE_BYTES));
        assert_eq!(l.push(swarm(), signed(&big)), Err(SnippetError::TooLarge));
        let keys: Vec<Key> = (0..=MAX_LOCAL_SNIPPETS)
            .map(|i| key_from_seed(&[u8::try_from(i).unwrap(); 32]))
            .collect();
        let push = |l: &mut LocalSnippets, k: &Key| {
            let s = SignedSnippet::sign(&Signer::new(k).unwrap(), &a, T0);
            l.push(SwarmId::new(&k.public), s)
        };
        for k in &keys[..MAX_LOCAL_SNIPPETS] {
            push(&mut l, k).unwrap();
        }
        assert_eq!(
            push(&mut l, &keys[MAX_LOCAL_SNIPPETS]),
            Err(SnippetError::Full)
        );
        // an existing swarm still updates
        push(&mut l, &keys[0]).unwrap();
        assert!(!l.is_empty());
    }
}
