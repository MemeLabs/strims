// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A swarm: its id, options, chunk store, integrity verifier and epoch.
//!
//! # The publish seam
//!
//! The [`Swarm`] owns the store, the verifier and the epoch, and every
//! chunk reaches the store through it:
//!
//! - [`Swarm::handle_data`] (a peer's chunk) verifies and then publishes;
//!   the [`Writer`](crate::Writer) publishes local chunks directly.
//!   Publishing stores the chunk and returns [`Published`], which says
//!   whether it was new.
//! - Every published bin is also queued for the scheduler, and
//!   [`Swarm::reset`] queues a reset. [`Swarm::poll_event`] yields
//!   [`SwarmEvent::Reset`] first, then [`SwarmEvent::Published`] in
//!   ascending bin order, then [`SwarmEvent::Readable`].
//! - The owner drains the queue after every call that can publish or reset
//!   (`handle_data`, `reset`, and the writer's `write`, `flush` and
//!   `reset`), before it handles the next message, so the scheduler sees
//!   each chunk before anything that follows it. Other store changes
//!   (`set_offset`, `close`, a cache import) only produce `Readable`.
//! - [`SwarmEvent::Readable`] tells the driver that local readers may have
//!   something new to read.
//!
//! The queue is a bin map, so it is bounded by the live window: adjacent
//! bins published between two drains come out merged into the largest
//! aligned bins, and bins that fell out of the window before the drain are
//! dropped.

use strims_base::time::Time;
use strims_binmap::{Bin, Map};
use strims_proto::types::Cache;

use crate::codec::{
    EpochOption, Handshake, MessageWriter, ProtocolOption, ProtocolOptions, ReaderConfig,
};
use crate::epoch::{Epoch, EpochError};
use crate::integrity::{self, ChannelVerifier, SwarmVerifier};
use crate::options::SwarmOptions;
use crate::store::{self, Buffer, BufferReader};
use crate::swarm_id::SwarmId;
use crate::uri::Uri;
use crate::{MINIMUM_PROTOCOL_VERSION, PROTOCOL_VERSION};

/// Swarm errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum SwarmError {
    #[error("invalid swarm options: {0}")]
    InvalidOptions(&'static str),
    /// The cache was made for another URI (swarm id or options).
    #[error("cache import failed: incompatible swarm options")]
    IncompatibleCache,
    /// A writer used with a swarm it didn't create.
    #[error("writer used with another swarm")]
    WrongSwarm,
    #[error(transparent)]
    Store(#[from] store::Error),
    #[error(transparent)]
    Integrity(#[from] integrity::Error),
    #[error(transparent)]
    Epoch(#[from] EpochError),
}

/// What the scheduler and driver learn from a swarm. See the module docs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwarmEvent {
    /// The swarm was reset (epoch change or writer reset): reset the
    /// scheduler.
    Reset,
    /// Chunks were stored: hand them to the scheduler.
    Published(Bin),
    /// Readers may have new data or errors.
    Readable,
}

/// The result of publishing a chunk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Published {
    pub bin: Bin,
    /// False if every chunk of `bin` was already stored (or is before the
    /// window).
    pub new: bool,
}

#[derive(Debug)]
struct Events {
    reset: bool,
    /// Bound: pruned to the store's window on every publish.
    published: Map,
    readable_seen: u64,
}

/// A swarm. See the module docs.
#[derive(Debug)]
pub struct Swarm {
    id: SwarmId,
    options: SwarmOptions,
    store: Buffer,
    pub(crate) verifier: SwarmVerifier,
    epoch: Epoch,
    events: Events,
}

/// The handshake options a swarm sends.
#[derive(Clone, Copy, Debug)]
pub struct SwarmHandshake<'a> {
    options: [ProtocolOption<'a>; 11],
    len: usize,
}

impl<'a> SwarmHandshake<'a> {
    pub fn options(&self) -> &[ProtocolOption<'a>] {
        self.options.get(..self.len).unwrap_or_default()
    }

    /// The handshake message. Its channel id field is unused and always 0:
    /// frame headers address channels.
    pub fn handshake(&self) -> Handshake<'_> {
        Handshake {
            channel_id: 0,
            options: ProtocolOptions::from_slice(self.options()),
        }
    }
}

fn validate(o: &SwarmOptions) -> Result<(), SwarmError> {
    let u32_max = u32::MAX as usize;
    if o.chunk_size == 0 || o.chunk_size > u32_max {
        return Err(SwarmError::InvalidOptions("chunk size"));
    }
    if !o.live_window.is_power_of_two() || o.live_window > 1 << 31 {
        return Err(SwarmError::InvalidOptions("live window"));
    }
    if o.chunks_per_signature == 0 || o.chunks_per_signature > u32_max {
        return Err(SwarmError::InvalidOptions("chunks per signature"));
    }
    if o.stream_count == 0 || o.stream_count > usize::from(u16::MAX) {
        return Err(SwarmError::InvalidOptions("stream count"));
    }
    // channel decoding sizes integrity messages by the hash function
    // whatever the protection method, so it must be supported
    if o.integrity.merkle_hash_tree_function.hash_size().is_none() {
        return Err(SwarmError::InvalidOptions("merkle hash tree function"));
    }
    Ok(())
}

impl Swarm {
    /// A swarm with `options`, whose zero fields are filled from
    /// [`SwarmOptions::default_options`]. Options the handshake can't carry
    /// (sizes of 0 or past `u32`, stream counts outside 1..=65535, a window
    /// that isn't a power of two up to 2^31) or the verifier can't serve
    /// are rejected.
    /// The live window buffer is allocated here, unless its chunks' bytes
    /// are [generated](crate::store::ChunkBytes::Generated). With ed25519
    /// the id must be the 32 byte public key.
    pub fn new(id: SwarmId, options: SwarmOptions) -> Result<Swarm, SwarmError> {
        let options = options.assign_defaults(&SwarmOptions::default_options());
        validate(&options)?;

        let store = Buffer::with_chunk_bytes(
            options.live_window,
            options.chunk_size,
            options.buffer_layout,
            options.chunk_bytes,
        )?;
        let ivo = options.integrity_verifier_options();
        let sv = ivo
            .verifier_options
            .live_signature_algorithm
            .verifier(id.as_bytes())?;
        let verifier = SwarmVerifier::new(sv.clone(), &ivo)?;

        Ok(Swarm {
            id,
            options,
            store,
            verifier,
            epoch: Epoch::new(sv),
            events: Events {
                reset: false,
                published: Map::new(),
                readable_seen: 0,
            },
        })
    }

    /// A swarm with [`SwarmOptions::default_options`].
    pub fn new_default(id: SwarmId) -> Result<Swarm, SwarmError> {
        Swarm::new(id, SwarmOptions::default_options())
    }

    /// The swarm id: its public key.
    pub fn id(&self) -> &SwarmId {
        &self.id
    }

    /// The options, with defaults filled in.
    pub fn options(&self) -> &SwarmOptions {
        &self.options
    }

    /// The swarm's URI: its id and options.
    pub fn uri(&self) -> Uri {
        Uri::new(self.id.clone(), self.options.uri_options())
    }

    pub fn store(&self) -> &Buffer {
        &self.store
    }

    pub fn verifier(&self) -> &SwarmVerifier {
        &self.verifier
    }

    pub fn epoch(&self) -> &Epoch {
        &self.epoch
    }

    /// The decoder settings for this swarm's channels.
    pub fn reader_config(&self) -> ReaderConfig {
        let o = &self.options.integrity;
        ReaderConfig {
            chunk_size: self.options.chunk_size,
            integrity_hash_size: o.merkle_hash_tree_function.hash_size().unwrap_or(0),
            integrity_signature_size: o.live_signature_algorithm.signature_size().unwrap_or(0),
        }
    }

    /// The options of the handshake this swarm sends, epoch included once
    /// it has one.
    pub fn handshake_options(&self) -> SwarmHandshake<'_> {
        let o = &self.options;
        let mut options = [ProtocolOption::Version(0); 11];
        let list = [
            ProtocolOption::SwarmIdentifier(self.id.as_bytes()),
            ProtocolOption::Version(PROTOCOL_VERSION),
            ProtocolOption::MinimumVersion(MINIMUM_PROTOCOL_VERSION),
            ProtocolOption::LiveWindow(o.live_window as u32),
            ProtocolOption::ChunkSize(o.chunk_size as u32),
            ProtocolOption::ContentIntegrityProtectionMethod(o.integrity.protection_method.0),
            ProtocolOption::MerkleHashTreeFunction(o.integrity.merkle_hash_tree_function.0),
            ProtocolOption::LiveSignatureAlgorithm(o.integrity.live_signature_algorithm.0),
            ProtocolOption::ChunksPerSignature(o.chunks_per_signature as u32),
            ProtocolOption::StreamCount(o.stream_count as u16),
        ];
        let mut len = 0;
        for (dst, src) in options.iter_mut().zip(list) {
            *dst = src;
            len += 1;
        }
        let (t, sig) = self.epoch.value();
        if let Some(e) = EpochOption::new(t, sig)
            && let Some(dst) = options.get_mut(len)
        {
            *dst = ProtocolOption::Epoch(e);
            len += 1;
        }
        SwarmHandshake { options, len }
    }

    /// A reader at the start of the window.
    pub fn reader(&self) -> BufferReader {
        BufferReader::new(&self.store)
    }

    /// [`BufferReader::read`] on this swarm's store.
    pub fn read(&self, r: &mut BufferReader, p: &mut [u8]) -> Result<usize, store::Error> {
        r.read(&self.store, p)
    }

    /// [`BufferReader::recover`] on this swarm's store.
    pub fn recover_reader(&mut self, r: &mut BufferReader) -> Result<u64, store::Error> {
        r.recover(&mut self.store)
    }

    /// Sets the read offset. The scheduler calls this once it knows where
    /// the stream is. Bins at or past [`MAX_BIN`](crate::MAX_BIN) are
    /// ignored.
    pub fn set_offset(&mut self, b: Bin) {
        if b < crate::MAX_BIN {
            self.store.set_offset(b);
        }
    }

    /// Loads a cache, whose URI must match this swarm's and whose epoch the
    /// swarm key signed. A refused URI or epoch leaves the swarm unchanged.
    /// The store takes only the data the cache's signatures cover.
    pub fn import_cache(&mut self, c: &Cache) -> Result<(), SwarmError> {
        if c.uri != self.uri().to_string() {
            return Err(SwarmError::IncompatibleCache);
        }
        self.epoch.import_cache(c.epoch.as_ref())?;
        let verified = self.verifier.import_cache(c)?;
        let data = c.data.get(..verified).unwrap_or(&c.data);
        self.store.import_cache(data)?;
        Ok(())
    }

    /// The swarm as a cache. Fails once the window has moved past chunk 0.
    pub fn export_cache(&self) -> Result<Cache, SwarmError> {
        let data = self.store.export_cache()?;
        Ok(Cache {
            uri: self.uri().to_string(),
            epoch: Some(self.epoch.export_cache()),
            integrity: Some(self.verifier.export_cache()),
            data,
            ..Default::default()
        })
    }

    /// Closes the store: readers get [`store::Error::Closed`].
    pub fn close(&mut self) {
        self.store.close();
    }

    /// Adopts a newer signed epoch from a peer's handshake. On `Ok(true)`
    /// the caller must [`Swarm::reset`].
    pub fn sync_epoch(&mut self, t: Time, sig: &[u8]) -> Result<bool, EpochError> {
        self.epoch.sync(t, sig)
    }

    /// Drops all chunks and integrity state and queues
    /// [`SwarmEvent::Reset`].
    pub fn reset(&mut self) {
        self.store.reset();
        self.verifier.reset();
        self.events.reset = true;
        self.events.published = Map::new();
    }

    /// A verifier for a new peer channel.
    pub fn channel_verifier(&self) -> ChannelVerifier {
        self.verifier.channel_verifier()
    }

    /// An `Integrity` message from a peer.
    pub fn handle_integrity(&self, cv: &mut ChannelVerifier, b: Bin, hash: &[u8]) {
        cv.chunk_verifier(b).set_integrity(b, hash);
    }

    /// A `SignedIntegrity` message from a peer.
    pub fn handle_signed_integrity(&self, cv: &mut ChannelVerifier, b: Bin, t: Time, sig: &[u8]) {
        cv.chunk_verifier(b).set_signed_integrity(b, t, sig);
    }

    /// A `Data` message from a peer that the scheduler expected: verifies
    /// it against the integrity messages that preceded it and publishes it.
    /// An error means the data was invalid; the caller tells the scheduler
    /// either way.
    pub fn handle_data(
        &mut self,
        cv: &mut ChannelVerifier,
        b: Bin,
        data: &[u8],
    ) -> Result<Published, SwarmError> {
        cv.chunk_verifier(b).verify(&mut self.verifier, b, data)?;
        self.publish(b, data)
    }

    /// The integrity messages for sending `b` to a peer that has
    /// `peer_have`.
    pub fn write_integrity(
        &self,
        b: Bin,
        peer_have: &Map,
        w: &mut impl MessageWriter,
    ) -> Result<usize, integrity::Error> {
        self.verifier.write_integrity(b, peer_have, w)
    }

    /// The `Data` message for `b`.
    pub fn write_data(
        &self,
        b: Bin,
        t: Time,
        w: &mut impl MessageWriter,
    ) -> Result<usize, store::Error> {
        self.store.write_data(b, t, w)
    }

    /// Stores a verified chunk and queues it for the scheduler.
    pub(crate) fn publish(&mut self, b: Bin, data: &[u8]) -> Result<Published, SwarmError> {
        let new = !self.store.bins_filled_at(b);
        self.store.consume(b, data)?;
        self.events.published.set(b);
        self.events
            .published
            .reset_before(self.store.window_start());
        Ok(Published { bin: b, new })
    }

    /// The next notification for the runner: resets first, then published
    /// bins in ascending order, then readability. See the module docs.
    pub fn poll_event(&mut self) -> Option<SwarmEvent> {
        if self.events.reset {
            self.events.reset = false;
            return Some(SwarmEvent::Reset);
        }
        if let Some(b) = self.events.published.iterate_filled().next() {
            self.events.published.reset(b);
            return Some(SwarmEvent::Published(b));
        }
        let seq = self.store.readable_seq();
        if seq != self.events.readable_seen {
            self.events.readable_seen = seq;
            return Some(SwarmEvent::Readable);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    //! Swarm tests: caches, the publish queue, option validation and
    //! handshakes.
    #![allow(clippy::unwrap_used, clippy::indexing_slicing, clippy::panic)]

    use super::*;
    use crate::integrity::{LiveSignatureAlgorithm, ProtectionMethod, VerifierOptions};
    use crate::writer::{Writer, WriterOptions};
    use rand_core::Rng;
    use strims_base::rng::det_rng;

    fn now() -> Time {
        Time::from_unix_millis(1_700_000_000_000)
    }

    fn drain(s: &mut Swarm) -> Vec<SwarmEvent> {
        core::iter::from_fn(|| s.poll_event()).collect()
    }

    fn read_all(s: &Swarm, r: &mut BufferReader, n: usize) -> Vec<u8> {
        let mut out = vec![0u8; n];
        let mut off = 0;
        while off < n {
            match s.read(r, &mut out[off..]) {
                Ok(m) => off += m,
                Err(store::Error::WouldBlock) => break,
                Err(e) => panic!("{e}"),
            }
        }
        out.truncate(off);
        out
    }

    /// A cache round-trips into a fresh swarm. A swarm refuses a cache with
    /// a forged epoch or with other options.
    #[test]
    fn cache() {
        let key = strims_crypto::key_from_seed(&[11; 32]);
        let opt = SwarmOptions::default_options();
        let (mut w, mut ws) = Writer::new(
            WriterOptions {
                swarm_options: opt.clone(),
                key: key.clone(),
            },
            now(),
        )
        .unwrap();

        let mut b = vec![0u8; opt.chunk_size * opt.chunks_per_signature * 8];
        det_rng(1).fill_bytes(&mut b);
        assert_eq!(w.write(&mut ws, now(), &b), Ok(b.len()));

        let c = ws.export_cache().unwrap();

        // a forged epoch is refused before anything is imported
        let mut forged = c.clone();
        forged.epoch.as_mut().unwrap().timestamp += 1_000_000;
        let mut s = Swarm::new_default(SwarmId::new(&key.public)).unwrap();
        assert_eq!(
            s.import_cache(&forged),
            Err(SwarmError::Epoch(EpochError::InvalidSignature))
        );
        assert!(s.epoch().value().0.is_nil());
        assert!(s.store().is_empty());

        // import
        let mut s = Swarm::new_default(SwarmId::new(&key.public)).unwrap();
        s.import_cache(&c).unwrap();
        assert_eq!(s.epoch().value(), ws.epoch().value());

        let mut r = s.reader();
        assert_eq!(read_all(&s, &mut r, b.len()), b);

        // a swarm with other options refuses the cache
        let mut other = Swarm::new(
            SwarmId::new(&key.public),
            SwarmOptions {
                chunk_size: 2048,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(other.import_cache(&c), Err(SwarmError::IncompatibleCache));
    }

    /// A cache's data past its signed munros, or a cache with no
    /// signatures at all, leaves nothing unverified in the store.
    #[test]
    fn cache_imports_only_signed_data() {
        let key = strims_crypto::key_from_seed(&[12; 32]);
        let opt = SwarmOptions::default_options();
        let (mut w, mut ws) = Writer::new(
            WriterOptions {
                swarm_options: opt.clone(),
                key: key.clone(),
            },
            now(),
        )
        .unwrap();
        let munro = opt.chunk_size * opt.chunks_per_signature;
        let mut b = vec![0u8; munro * 2];
        det_rng(2).fill_bytes(&mut b);
        assert_eq!(w.write(&mut ws, now(), &b), Ok(b.len()));
        let c = ws.export_cache().unwrap();

        let mut padded = c.clone();
        padded.data.extend(std::iter::repeat_n(0xa5, munro));
        let mut s = Swarm::new_default(SwarmId::new(&key.public)).unwrap();
        s.import_cache(&padded).unwrap();
        let mut r = s.reader();
        assert_eq!(read_all(&s, &mut r, b.len() + munro), b);

        let mut unsigned = c.clone();
        if let Some(m) = unsigned
            .integrity
            .as_mut()
            .and_then(|i| i.merkle_integrity.as_mut())
        {
            m.timestamps.clear();
            m.signatures.clear();
        }
        let mut s = Swarm::new_default(SwarmId::new(&key.public)).unwrap();
        s.import_cache(&unsigned).unwrap();
        assert!(s.store().is_empty());
    }

    /// Published chunks reach the store and the scheduler queue, and resets
    /// reach both.
    #[test]
    fn publish_and_reset_reach_store_and_queue() {
        let key = strims_crypto::key_from_seed(&[12; 32]);
        let (mut w, mut s) = Writer::new(
            WriterOptions {
                swarm_options: SwarmOptions {
                    chunk_size: 1024,
                    integrity: VerifierOptions {
                        protection_method: ProtectionMethod::NONE,
                        ..Default::default()
                    },
                    ..Default::default()
                },
                key,
            },
            now(),
        )
        .unwrap();
        assert_eq!(drain(&mut s), vec![SwarmEvent::Readable]);

        let data = vec![7u8; 1024 * 3];
        assert_eq!(w.write(&mut s, now(), &data), Ok(data.len()));
        assert_eq!(w.flush(&mut s, now()), Ok(()));
        // three chunks, merged into the largest aligned bins
        assert_eq!(
            drain(&mut s),
            vec![
                SwarmEvent::Published(Bin(1)),
                SwarmEvent::Published(Bin(4)),
                SwarmEvent::Readable
            ]
        );
        let mut r = s.reader();
        assert_eq!(read_all(&s, &mut r, 10_000), data);

        // one chunk at a time
        w.write(&mut s, now(), &[1; 1024]).unwrap();
        assert_eq!(drain(&mut s)[0], SwarmEvent::Published(Bin(6)));

        w.reset(&mut s, now() + strims_base::time::Duration::SECOND)
            .unwrap();
        assert_eq!(drain(&mut s), vec![SwarmEvent::Reset, SwarmEvent::Readable]);
        assert!(s.store().is_empty());
        assert_eq!(s.read(&mut r, &mut [0; 1]), Err(store::Error::StreamReset));
        w.write(&mut s, now(), &[2; 1024]).unwrap();
        assert_eq!(drain(&mut s)[0], SwarmEvent::Published(Bin(0)));
    }

    #[test]
    fn publish_reports_new_and_validates() {
        let mut s = Swarm::new(
            SwarmId::new(&[0; 32]),
            SwarmOptions {
                chunk_size: 4,
                live_window: 4,
                chunks_per_signature: 1,
                ..Default::default()
            },
        )
        .unwrap();
        s.set_offset(Bin(0));
        assert_eq!(
            s.publish(Bin(2), &[1; 4]),
            Ok(Published {
                bin: Bin(2),
                new: true
            })
        );
        assert_eq!(
            s.publish(Bin(2), &[1; 4]),
            Ok(Published {
                bin: Bin(2),
                new: false
            })
        );
        assert_eq!(
            s.publish(Bin(1), &[1; 8]),
            Ok(Published {
                bin: Bin(1),
                new: true
            })
        );
        assert_eq!(
            s.publish(Bin(1), &[1; 7]),
            Err(store::Error::DataLength.into())
        );
        assert_eq!(
            s.publish(Bin(7), &[1; 32]),
            Err(store::Error::BinOutOfBounds.into())
        );
        assert_eq!(
            s.publish(crate::MAX_BIN, &[1; 4]),
            Err(store::Error::BinOutOfBounds.into())
        );
        // the queue is pruned to the window
        for o in 2..12u64 {
            s.publish(Bin(2 * o), &[0; 4]).unwrap();
        }
        assert_eq!(
            drain(&mut s),
            vec![SwarmEvent::Published(Bin(19)), SwarmEvent::Readable]
        );
    }

    #[test]
    fn options_are_validated() {
        let id = SwarmId::new(&[0; 32]);
        let bad = |o: SwarmOptions| Swarm::new(id.clone(), o).err();
        assert_eq!(
            bad(SwarmOptions {
                live_window: 1000,
                ..Default::default()
            }),
            Some(SwarmError::InvalidOptions("live window"))
        );
        assert_eq!(
            bad(SwarmOptions {
                stream_count: 1 << 16,
                ..Default::default()
            }),
            Some(SwarmError::InvalidOptions("stream count"))
        );
        assert_eq!(
            bad(SwarmOptions {
                integrity: VerifierOptions {
                    merkle_hash_tree_function: crate::integrity::MerkleHashTreeFunction::MD5,
                    ..Default::default()
                },
                ..Default::default()
            }),
            Some(SwarmError::InvalidOptions("merkle hash tree function"))
        );
        assert_eq!(
            bad(SwarmOptions {
                chunks_per_signature: 1 << 17,
                ..Default::default()
            }),
            Some(SwarmError::Integrity(integrity::Error::InvalidOptions))
        );
        assert_eq!(
            Swarm::new(SwarmId::new(&[0; 31]), SwarmOptions::default()).err(),
            Some(SwarmError::Integrity(integrity::Error::InvalidKey))
        );
        assert!(
            Swarm::new(
                SwarmId::new(&[0; 3]),
                SwarmOptions {
                    integrity: VerifierOptions {
                        live_signature_algorithm: LiveSignatureAlgorithm::NONE,
                        ..Default::default()
                    },
                    ..Default::default()
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn handshake_carries_swarm_options_and_epoch() {
        let key = strims_crypto::key_from_seed(&[13; 32]);
        let mut s = Swarm::new_default(SwarmId::new(&key.public)).unwrap();
        let h = s.handshake_options();
        assert_eq!(h.options().len(), 10, "no epoch yet");
        let sig = LiveSignatureAlgorithm::ED25519
            .signer(&key.private)
            .unwrap()
            .sign(strims_crypto::Domain::PpsppEpoch, now(), &[]);
        assert_eq!(s.sync_epoch(now(), sig.as_slice()), Ok(true));
        let h = s.handshake_options();
        let o = h.handshake().options.summary();
        assert_eq!(o.swarm_identifier, Some(&key.public[..]));
        assert_eq!(o.version, Some(2));
        assert_eq!(o.minimum_version, Some(2));
        assert_eq!(o.live_window, Some(1 << 16));
        assert_eq!(o.chunk_size, Some(1024));
        assert_eq!(o.chunks_per_signature, Some(64));
        assert_eq!(o.stream_count, Some(1));
        assert_eq!(o.content_integrity_protection_method, Some(2));
        assert_eq!(o.merkle_hash_tree_function, Some(4));
        assert_eq!(o.live_signature_algorithm, Some(2));
        assert_eq!(o.epoch.map(|e| e.timestamp), Some(now()));
    }
}
