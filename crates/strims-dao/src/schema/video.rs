// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The profile's video channels, and the configs of this device's video
//! ingress and HLS egress.
//!
//! All three stay on the device: none replicate, and each is observable.
//! A channel's owner says who streams to it and on which network
//! ([`channel_network_key`]):
//! - `Local`: the profile, on a network it belongs to;
//! - `LocalShare`: the holder of a network certificate, through this
//!   device's ingress;
//! - `RemoteShare`: the profile, through another device's ingress.
//!
//! Deleting a network deletes the channels on it and drops its key from
//! the ingress config ([`delete_for_network`]).
//!
//! Index keys are stored, so [`owner_key`] must keep deriving the same
//! bytes.
//!
//! # Stream keys
//!
//! A channel's stream key is unpadded base64url of its id as a uvarint,
//! then the ed25519 signature of its token by its owner's key: the
//! `Local` auth key, or the `LocalShare` certificate's key
//! ([`format_stream_key`], [`channel_by_stream_key`]).

use data_encoding::BASE64URL_NOPAD;
use rand_core::Rng;
use strims_crypto::sign::verify;
use strims_crypto::{
    CertificateChain, Domain, ED25519_SIGNATURE_SIZE, Signer, certificate_body, generate_key,
};
use strims_kv::Kv;
use strims_proto::network::v1::directory::ListingSnippet;
use strims_proto::types::Key;
use strims_proto::video::v1::video_channel::Owner;
use strims_proto::video::v1::{HlsEgressConfig, VideoChannel, VideoIngressConfig};

use crate::error::Error;
use crate::events::Record;
use crate::key::Namespace;
use crate::store::{Read, WriteTx};
use crate::table::{Index, Singleton, Table};

/// The profile's video channels.
pub struct VideoChannels;

impl VideoChannels {
    /// Channels by [`owner_key`]; at most one per owner.
    pub const BY_OWNER: Index<VideoChannel> = Index::unique(Namespace(7002), owner_key);
}

impl Table for VideoChannels {
    type Record = VideoChannel;
    const NS: Namespace = Namespace(7001);
    const INDEXES: &'static [Index<VideoChannel>] = &[VideoChannels::BY_OWNER];

    fn id(r: &VideoChannel) -> u64 {
        r.id
    }

    fn observe(r: &VideoChannel) -> Option<Record> {
        Some(Record::VideoChannel(r.clone()))
    }
}

const LOCAL: u8 = 1;
const LOCAL_SHARE: u8 = 2;
const REMOTE_SHARE: u8 = 3;

/// The key a channel's owner is indexed under:
/// - `Local`: 1, then the channel id as a uvarint;
/// - `LocalShare`: 2, the certificate's root key, then the certificate's
///   key (2 alone without a certificate);
/// - `RemoteShare`: 3, the network key, then the service key.
///
/// A channel without an owner is keyed by no bytes.
pub fn owner_key(c: &VideoChannel) -> Vec<u8> {
    let mut k = Vec::new();
    match &c.owner {
        Some(Owner::Local(_)) => {
            k.push(LOCAL);
            put_uvarint(&mut k, c.id);
        }
        Some(Owner::LocalShare(s)) => {
            k.push(LOCAL_SHARE);
            if let Some(c) = s
                .certificate
                .as_ref()
                .and_then(|c| CertificateChain::decode(c).ok())
            {
                k.extend_from_slice(c.network_key());
                k.extend_from_slice(&c.key);
            }
        }
        Some(Owner::RemoteShare(r)) => {
            k.push(REMOTE_SHARE);
            k.extend_from_slice(&r.network_key);
            k.extend_from_slice(&r.service_key);
        }
        None => {}
    }
    k
}

/// The key of the network channel `c` streams to: a `Local` or
/// `RemoteShare` owner's network key, or a `LocalShare` certificate's
/// root key. None without an owner, or for a `LocalShare` without a
/// certificate.
pub fn channel_network_key(c: &VideoChannel) -> Option<Vec<u8>> {
    match c.owner.as_ref()? {
        Owner::Local(l) => Some(l.network_key.clone()),
        Owner::LocalShare(s) => {
            let c = CertificateChain::decode(s.certificate.as_ref()?).ok()?;
            Some(c.network_key().to_vec())
        }
        Owner::RemoteShare(r) => Some(r.network_key.clone()),
    }
}

/// The size of a channel's token, the message its stream key signs.
pub const TOKEN_SIZE: usize = 32;

/// A channel of `owner`, listed with `snippet`, with a new id, ed25519
/// key and token. It isn't stored.
pub async fn new_channel<K: Kv>(
    tx: &mut WriteTx<K>,
    owner: Owner,
    snippet: ListingSnippet,
) -> Result<VideoChannel, Error> {
    let id = tx.generate_id().await?;
    let key = generate_key(tx.rng());
    let mut token = vec![0; TOKEN_SIZE];
    tx.rng().fill_bytes(&mut token);
    Ok(VideoChannel {
        id,
        key: Some(key),
        token,
        directory_listing_snippet: Some(snippet),
        owner: Some(owner),
    })
}

/// Deletes the channels on the network with key `network_key` (by
/// [`channel_network_key`]) and drops the key from the ingress config's
/// service network keys.
pub async fn delete_for_network<K: Kv>(
    tx: &mut WriteTx<K>,
    network_key: &[u8],
) -> Result<(), Error> {
    // Channels are local to this device and few, so a scan finds the
    // network's.
    for c in VideoChannels::get_all(tx).await? {
        if channel_network_key(&c).as_deref() == Some(network_key) {
            VideoChannels::delete(tx, c.id).await?;
        }
    }
    IngressConfig::transform(tx, |c| {
        c.service_network_keys.retain(|k| k != network_key);
        Ok(())
    })
    .await?;
    Ok(())
}

/// The longest uvarint: a `u64` in 7-bit groups.
const MAX_UVARINT_LEN: usize = 10;

/// The most bytes a stream key encodes: an id and a signature.
const MAX_STREAM_KEY_BYTES: usize = MAX_UVARINT_LEN + ED25519_SIGNATURE_SIZE;

fn put_uvarint(out: &mut Vec<u8>, mut v: u64) {
    while v >= 0x80 {
        out.push(v as u8 | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}

/// The uvarint at the start of `b` and its length. None if `b` ends
/// within it or it exceeds 64 bits. Encodings longer than needed are
/// accepted.
fn get_uvarint(b: &[u8]) -> Option<(u64, usize)> {
    let mut x = 0u64;
    for (i, &byte) in b.iter().enumerate().take(MAX_UVARINT_LEN) {
        let shift = 7 * i as u32;
        if byte < 0x80 {
            if i == MAX_UVARINT_LEN - 1 && byte > 1 {
                return None;
            }
            return Some((x | (u64::from(byte) << shift), i + 1));
        }
        x |= u64::from(byte & 0x7f) << shift;
    }
    None
}

/// The stream key of channel `id`, whose token is `token`, signed with
/// its owner's `key` in [`Domain::VideoStreamKey`]. Fails with
/// [`Error::Invalid`] if `key` holds no ed25519 private key.
pub fn format_stream_key(id: u64, token: &[u8], key: &Key) -> Result<String, Error> {
    let signature = Signer::from_private(&key.private)
        .map_err(|_| Error::Invalid("stream key signing key"))?
        .sign(Domain::VideoStreamKey, &[token]);
    let mut b = Vec::with_capacity(MAX_STREAM_KEY_BYTES);
    put_uvarint(&mut b, id);
    b.extend_from_slice(&signature);
    Ok(BASE64URL_NOPAD.encode(&b))
}

/// Why a stream key didn't parse.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum StreamKeyError {
    /// The key isn't canonical unpadded base64url.
    #[error("stream key is not base64url")]
    Encoding,
    /// The channel id is truncated or exceeds 64 bits.
    #[error("stream key has no valid channel id")]
    Id,
    /// The key is too long, or the bytes after the id aren't a signature.
    #[error("stream key has the wrong length")]
    Length,
}

/// The channel id and signature in stream key `key`. A key longer than
/// any stream key fails with [`StreamKeyError::Length`] undecoded.
pub fn parse_stream_key(key: &str) -> Result<(u64, [u8; ED25519_SIGNATURE_SIZE]), StreamKeyError> {
    if key.len() > BASE64URL_NOPAD.encode_len(MAX_STREAM_KEY_BYTES) {
        return Err(StreamKeyError::Length);
    }
    let b = BASE64URL_NOPAD
        .decode(key.as_bytes())
        .map_err(|_| StreamKeyError::Encoding)?;
    let (id, n) = get_uvarint(&b).ok_or(StreamKeyError::Id)?;
    let signature = b
        .get(n..)
        .and_then(|s| <[u8; ED25519_SIGNATURE_SIZE]>::try_from(s).ok())
        .ok_or(StreamKeyError::Length)?;
    Ok((id, signature))
}

/// The channel stream key `key` names, if the channel's owner signed its
/// token: the `Local` auth key or the `LocalShare` certificate's key.
/// None if `key` doesn't parse, names no channel, or its signature
/// doesn't verify. A `RemoteShare` channel's ingress runs on another
/// device, so no stream key here names one.
pub async fn channel_by_stream_key<R: Read>(
    tx: &mut R,
    key: &str,
) -> Result<Option<VideoChannel>, Error> {
    let Ok((id, signature)) = parse_stream_key(key) else {
        return Ok(None);
    };
    let Some(c) = VideoChannels::get(tx, id).await? else {
        return Ok(None);
    };
    let owner_key = match &c.owner {
        Some(Owner::Local(l)) => l.auth_key.clone(),
        Some(Owner::LocalShare(s)) => s
            .certificate
            .as_ref()
            .and_then(|cert| certificate_body(cert).ok())
            .map(|b| b.key)
            .unwrap_or_default(),
        Some(Owner::RemoteShare(_)) | None => return Ok(None),
    };
    if !verify(Domain::VideoStreamKey, &owner_key, &[&c.token], &signature) {
        return Ok(None);
    }
    Ok(Some(c))
}

/// The address the video ingress listens on by default.
pub const DEFAULT_INGRESS_SERVER_ADDR: &str = "127.0.0.1:1935";

/// This device's video ingress config. It defaults to disabled, with
/// [`DEFAULT_INGRESS_SERVER_ADDR`].
pub struct IngressConfig;

impl Singleton for IngressConfig {
    type Record = VideoIngressConfig;
    const NS: Namespace = Namespace(7003);

    fn default_value() -> Option<VideoIngressConfig> {
        Some(VideoIngressConfig {
            server_addr: DEFAULT_INGRESS_SERVER_ADDR.to_owned(),
            ..Default::default()
        })
    }

    fn observe(r: &VideoIngressConfig) -> Option<Record> {
        Some(Record::VideoIngressConfig(r.clone()))
    }
}

/// This device's HLS egress config. It defaults to disabled.
pub struct HlsConfig;

impl Singleton for HlsConfig {
    type Record = HlsEgressConfig;
    const NS: Namespace = Namespace(7004);

    fn default_value() -> Option<HlsEgressConfig> {
        Some(HlsEgressConfig::default())
    }

    fn observe(r: &HlsEgressConfig) -> Option<Record> {
        Some(Record::HlsEgressConfig(r.clone()))
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used, clippy::indexing_slicing)]

    use prost::Message;
    use strims_proto::types::{Certificate, CertificateBody};
    use strims_proto::video::v1::video_channel::{Local, LocalShare, RemoteShare};

    use super::*;

    fn channel(id: u64, owner: Owner) -> VideoChannel {
        VideoChannel {
            id,
            owner: Some(owner),
            ..Default::default()
        }
    }

    #[test]
    fn owner_keys_by_kind() {
        let local = channel(300, Owner::Local(Local::default()));
        assert_eq!(owner_key(&local), [1, 0xac, 0x02]);
        // Unsigned: neither function checks signatures.
        let root = Certificate {
            body: CertificateBody {
                key: b"root".to_vec(),
                serial_number: b"r".to_vec(),
                ..Default::default()
            }
            .encode_to_vec(),
            ..Default::default()
        };
        let leaf = |issuer_serial_number: &[u8]| Certificate {
            body: CertificateBody {
                key: b"leaf".to_vec(),
                issuer_serial_number: issuer_serial_number.to_vec(),
                ..Default::default()
            }
            .encode_to_vec(),
            parent: Some(Box::new(root.clone())),
            ..Default::default()
        };
        let share_of = |cert| {
            channel(
                1,
                Owner::LocalShare(LocalShare {
                    certificate: Some(cert),
                }),
            )
        };
        let share = share_of(leaf(b"r"));
        assert_eq!(owner_key(&share), b"\x02rootleaf");
        assert_eq!(channel_network_key(&share).as_deref(), Some(&b"root"[..]));
        let remote = channel(
            1,
            Owner::RemoteShare(RemoteShare {
                network_key: b"net".to_vec(),
                service_key: b"svc".to_vec(),
                ..Default::default()
            }),
        );
        assert_eq!(owner_key(&remote), b"\x03netsvc");
        assert_eq!(channel_network_key(&remote).as_deref(), Some(&b"net"[..]));
        let unowned = VideoChannel::default();
        assert!(owner_key(&unowned).is_empty());
        assert_eq!(channel_network_key(&unowned), None);
        let bare_share = channel(1, Owner::LocalShare(LocalShare::default()));
        assert_eq!(owner_key(&bare_share), [2]);
        assert_eq!(channel_network_key(&bare_share), None);
        // a chain whose parent isn't the issuer its leaf names
        let broken_share = share_of(leaf(b"other"));
        assert_eq!(owner_key(&broken_share), [2]);
        assert_eq!(channel_network_key(&broken_share), None);
    }

    #[test]
    fn uvarints_round_trip_at_every_length() {
        for shift in 0..64 {
            for v in [1u64 << shift, (1u64 << shift) - 1, u64::MAX >> shift] {
                let mut b = Vec::new();
                put_uvarint(&mut b, v);
                assert!(b.len() <= MAX_UVARINT_LEN);
                assert_eq!(get_uvarint(&b), Some((v, b.len())), "{v}");
            }
        }
    }

    #[test]
    fn video_records_stay_local() {
        assert!(VideoChannels::REPLICATION.is_none());
        assert!(IngressConfig::REPLICATION.is_none());
        assert!(HlsConfig::REPLICATION.is_none());
    }
}
