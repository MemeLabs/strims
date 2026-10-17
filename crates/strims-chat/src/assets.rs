// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Asset bundles: a chat server's room, icon, emotes, modifiers and tags as
//! records of its asset stream.
//!
//! # Stream contract
//!
//! - The stream starts, and starts over after each reset, with a unified
//!   bundle (`is_delta` false) holding the room, the icon, every enabled
//!   emote and every modifier and tag.
//! - Each later record is a delta (`is_delta` true) holding the emotes,
//!   modifiers and tags whose content changed, `room` when the server
//!   record changed and `icon` when the icon changed. `removed_ids` lists
//!   the ids of entries no longer present: deleted emotes, modifiers and
//!   tags, disabled emotes, and the icon, by its id (the server's), when
//!   it's deleted. Record ids are unique across a profile's records, so one
//!   list serves every kind.
//! - A reader applies a bundle's removals, then its entries, room and icon
//!   ([`fold`]).
//! - Records are framed in [`ASSET_FRAME_BYTES`] frames, and the stream
//!   holds at most [`ASSET_WINDOW_BYTES`] of them, so a reader that starts
//!   at the swarm's window tail reads it from its unified bundle on.
//!   [`AssetPublisher`] starts the stream over with a unified bundle before
//!   a delta would take it past that.
//!
//! [`split`] cuts a bundle into parts of a bounded size for responses with
//! a size cap.

use std::collections::{BTreeMap, BTreeSet};

use prost::Message;
use strims_chunkstream::{RecordEncoder, uvarint_len};
use strims_crypto::crc32_message;
use strims_proto::chat::v1::{AssetBundle, Emote, Modifier, Server, ServerIcon, Tag};

use crate::reader::RoomLimits;

/// The asset stream's frame size, in bytes.
pub const ASSET_FRAME_BYTES: usize = 128 * 1024;

/// The framed bytes the asset stream holds at most: the asset swarm's live
/// window. A record writer for the stream needs a `max_record` of at least
/// this, since one record may take all of it.
pub const ASSET_WINDOW_BYTES: usize = 32 * 1024 * 1024;

const _: () = assert!(
    ASSET_FRAME_BYTES > strims_chunkstream::HEADER_LEN
        && ASSET_FRAME_BYTES <= strims_chunkstream::MAX_FRAME_SIZE
);

/// The encoding of `is_delta: true`.
const DELTA_FLAG_BYTES: usize = 2;

/// A server's assets as stored.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Assets {
    pub server: Server,
    pub icon: Option<ServerIcon>,
    /// Every emote of the server; only enabled ones are published.
    pub emotes: Vec<Emote>,
    pub modifiers: Vec<Modifier>,
    pub tags: Vec<Tag>,
}

/// A bundle to append to the asset stream.
#[derive(Clone, Debug, PartialEq)]
pub struct AssetWrite {
    /// Start the stream over before writing `bundle`.
    pub reset: bool,
    pub bundle: AssetBundle,
}

/// Why [`AssetPublisher::sync`] wrote nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
pub enum AssetError {
    /// The unified bundle alone takes more than [`ASSET_WINDOW_BYTES`]
    /// framed.
    #[error(
        "asset bundle of {framed_len} framed bytes exceeds the {ASSET_WINDOW_BYTES} byte stream window"
    )]
    TooLarge { framed_len: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Server,
    Icon,
    Emote,
    Modifier,
    Tag,
}

/// Turns a server's assets into the records of its asset stream.
///
/// It remembers the CRC of each entry the stream holds, so each
/// [`sync`](Self::sync) writes only what changed, and the framed bytes the
/// stream holds, so the stream stays within [`ASSET_WINDOW_BYTES`].
#[derive(Clone, Debug)]
pub struct AssetPublisher {
    encoder: RecordEncoder,
    /// The CRC of each entry the stream holds, by kind and id. Bound: the
    /// entries of the last synced [`Assets`], which come from the local
    /// store.
    checksums: BTreeMap<(Kind, u64), u32>,
    /// Framed bytes of the records returned since the stream started over.
    size: usize,
    /// The next sync writes a unified bundle.
    force_unified: bool,
}

impl Default for AssetPublisher {
    fn default() -> Self {
        Self::new()
    }
}

impl AssetPublisher {
    /// A publisher for a new, empty stream.
    pub fn new() -> Self {
        // invariant: the frame size is in range, as the const assertion
        // above checks.
        #[allow(clippy::expect_used)]
        let encoder = RecordEncoder::new(ASSET_FRAME_BYTES).expect("asset frame size");
        AssetPublisher {
            encoder,
            checksums: BTreeMap::new(),
            size: 0,
            force_unified: false,
        }
    }

    /// The bundle that brings the stream up to `assets`, or `None` when
    /// nothing changed.
    ///
    /// The bundle is unified when `unified` is set, after
    /// [`reset`](Self::reset) or a failed sync, before anything was written,
    /// and when a delta would take the stream past [`ASSET_WINDOW_BYTES`].
    /// A unified bundle asks for a stream reset when the stream holds
    /// records. The caller writes the returned bundle as one record,
    /// framed in [`ASSET_FRAME_BYTES`] frames, and calls
    /// [`reset`](Self::reset) when that write fails.
    pub fn sync(
        &mut self,
        assets: &Assets,
        unified: bool,
    ) -> Result<Option<AssetWrite>, AssetError> {
        if !(unified || self.force_unified || self.checksums.is_empty()) {
            let bundle = self.bundle(assets, true);
            if is_empty_delta(&bundle) {
                return Ok(None);
            }
            if let Some(len) = self.framed_len(&bundle)
                && let Some(size) = self.size.checked_add(len)
                && size <= ASSET_WINDOW_BYTES
            {
                self.size = size;
                return Ok(Some(AssetWrite {
                    reset: false,
                    bundle,
                }));
            }
        }

        self.checksums.clear();
        let bundle = self.bundle(assets, false);
        match self.framed_len(&bundle) {
            Some(len) if len <= ASSET_WINDOW_BYTES => {
                let reset = self.size > 0;
                self.size = len;
                self.force_unified = false;
                Ok(Some(AssetWrite { reset, bundle }))
            }
            len => {
                self.checksums.clear();
                self.force_unified = true;
                Err(AssetError::TooLarge {
                    framed_len: len.unwrap_or(usize::MAX),
                })
            }
        }
    }

    /// Makes the next sync write a unified bundle that starts the stream
    /// over: the stream may no longer hold what this publisher returned.
    pub fn reset(&mut self) {
        self.force_unified = true;
    }

    /// The framed bytes of the records returned since the stream started
    /// over.
    pub fn stream_bytes(&self) -> usize {
        self.size
    }

    fn framed_len(&self, bundle: &AssetBundle) -> Option<usize> {
        self.encoder.encoded_len(bundle.encoded_len())
    }

    /// The entries of `assets` whose CRC differs from the stream's, and the
    /// ids the stream holds that `assets` lacks; records the new CRCs.
    fn bundle(&mut self, assets: &Assets, is_delta: bool) -> AssetBundle {
        let mut removed: BTreeSet<(Kind, u64)> = self.checksums.keys().copied().collect();
        let mut b = AssetBundle {
            is_delta,
            ..Default::default()
        };

        for e in assets.emotes.iter().filter(|e| e.enable) {
            if self.changed(&mut removed, (Kind::Emote, e.id), crc32_message(e)) {
                b.emotes.push(e.clone());
            }
        }
        for e in &assets.modifiers {
            if self.changed(&mut removed, (Kind::Modifier, e.id), crc32_message(e)) {
                b.modifiers.push(e.clone());
            }
        }
        for e in &assets.tags {
            if self.changed(&mut removed, (Kind::Tag, e.id), crc32_message(e)) {
                b.tags.push(e.clone());
            }
        }
        let server = &assets.server;
        if self.changed(
            &mut removed,
            (Kind::Server, server.id),
            crc32_message(server),
        ) {
            b.room = server.room.clone();
        }
        if let Some(icon) = &assets.icon
            && self.changed(&mut removed, (Kind::Icon, icon.id), crc32_message(icon))
        {
            b.icon = icon.image.clone();
        }

        for k in removed {
            self.checksums.remove(&k);
            b.removed_ids.push(k.1);
        }
        b
    }

    /// Marks `key` present and records its CRC. Returns whether the CRC
    /// differs from the stream's.
    fn changed(&mut self, removed: &mut BTreeSet<(Kind, u64)>, key: (Kind, u64), crc: u32) -> bool {
        removed.remove(&key);
        self.checksums.insert(key, crc) != Some(crc)
    }
}

fn is_empty_delta(b: &AssetBundle) -> bool {
    b.is_delta
        && b.removed_ids.is_empty()
        && b.room.is_none()
        && b.icon.is_none()
        && b.emotes.is_empty()
        && b.modifiers.is_empty()
        && b.tags.is_empty()
}

/// The encoded size of `m` as a length-delimited field numbered below 16.
fn field_len<M: Message>(m: &M) -> usize {
    let n = m.encoded_len();
    1 + prost::length_delimiter_len(n) + n
}

/// An emote, modifier or tag: an entry of a bundle, keyed by id.
trait Entry: Message {
    fn id(&self) -> u64;
}

impl Entry for Emote {
    fn id(&self) -> u64 {
        self.id
    }
}

impl Entry for Modifier {
    fn id(&self) -> u64 {
        self.id
    }
}

impl Entry for Tag {
    fn id(&self) -> u64 {
        self.id
    }
}

/// Applies `bundle` to `acc`, as a reader applies the records of an asset
/// stream. Returns the number of entries, removed ids, rooms and icons
/// left out for `limits`.
///
/// A unified `bundle` replaces `acc`. A delta removes the entries
/// `removed_ids` names, then upserts its emotes, modifiers and tags by id
/// (new ones are appended) and replaces the room and icon it holds. When
/// `acc` is a delta itself, it also keeps the removed ids, so it stays a
/// delta equal to the two applied in turn.
///
/// `acc` holds at most `limits.max_emotes`, `max_modifiers` and `max_tags`
/// entries and `max_removed_ids` removed ids, and its room, icon and
/// entries encode to at most `limits.max_asset_record` bytes; what `bundle`
/// adds beyond these is left out. The work is `O((n + m) log(n + m))` for
/// `n` entries in `acc` and `m` in `bundle`.
pub fn fold(acc: &mut AssetBundle, bundle: AssetBundle, limits: &RoomLimits) -> usize {
    if !bundle.is_delta {
        *acc = AssetBundle::default();
    }
    let AssetBundle {
        is_delta: _,
        removed_ids,
        room,
        emotes,
        modifiers,
        tags,
        icon,
    } = bundle;

    let mut f = Folder {
        bytes: acc.room.as_ref().map_or(0, field_len)
            + acc.icon.as_ref().map_or(0, field_len)
            + acc.emotes.iter().map(field_len).sum::<usize>()
            + acc.modifiers.iter().map(field_len).sum::<usize>()
            + acc.tags.iter().map(field_len).sum::<usize>(),
        max_bytes: limits.max_asset_record,
        dropped: 0,
    };

    if !removed_ids.is_empty() {
        let removed: BTreeSet<u64> = removed_ids.iter().copied().collect();
        f.remove(&mut acc.emotes, &removed);
        f.remove(&mut acc.modifiers, &removed);
        f.remove(&mut acc.tags, &removed);
        if acc.is_delta {
            let mut kept: BTreeSet<u64> = acc.removed_ids.iter().copied().collect();
            for id in removed_ids {
                if kept.contains(&id) {
                    continue;
                }
                if acc.removed_ids.len() >= limits.max_removed_ids {
                    f.dropped += 1;
                    continue;
                }
                kept.insert(id);
                acc.removed_ids.push(id);
            }
        }
    }

    if let Some(room) = room {
        f.replace(&mut acc.room, room);
    }
    if let Some(icon) = icon {
        f.replace(&mut acc.icon, icon);
    }
    f.upsert(&mut acc.emotes, emotes, limits.max_emotes);
    f.upsert(&mut acc.modifiers, modifiers, limits.max_modifiers);
    f.upsert(&mut acc.tags, tags, limits.max_tags);
    f.dropped
}

struct Folder {
    /// The encoded size of the accumulated room, icon and entries.
    bytes: usize,
    max_bytes: usize,
    dropped: usize,
}

impl Folder {
    fn remove<T: Entry>(&mut self, acc: &mut Vec<T>, removed: &BTreeSet<u64>) {
        acc.retain(|e| {
            let keep = !removed.contains(&e.id());
            if !keep {
                self.bytes = self.bytes.saturating_sub(field_len(e));
            }
            keep
        });
    }

    /// Whether replacing `old` bytes with `new` keeps within the bound;
    /// accounts the replacement when it does.
    fn admit(&mut self, old: usize, new: usize) -> bool {
        let bytes = self.bytes.saturating_sub(old).saturating_add(new);
        if bytes > self.max_bytes {
            self.dropped += 1;
            return false;
        }
        self.bytes = bytes;
        true
    }

    fn replace<T: Message>(&mut self, slot: &mut Option<T>, value: T) {
        if self.admit(slot.as_ref().map_or(0, field_len), field_len(&value)) {
            *slot = Some(value);
        }
    }

    fn upsert<T: Entry>(&mut self, acc: &mut Vec<T>, entries: Vec<T>, max: usize) {
        if entries.is_empty() {
            return;
        }
        let mut index: BTreeMap<u64, usize> =
            acc.iter().enumerate().map(|(i, e)| (e.id(), i)).collect();
        for e in entries {
            let new = field_len(&e);
            match index.get(&e.id()).and_then(|&i| acc.get_mut(i)) {
                Some(slot) => {
                    if self.admit(field_len(slot), new) {
                        *slot = e;
                    }
                }
                None => {
                    if acc.len() >= max {
                        self.dropped += 1;
                    } else if self.admit(0, new) {
                        index.insert(e.id(), acc.len());
                        acc.push(e);
                    }
                }
            }
        }
    }
}

/// A bundle cut into parts by [`split`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Split {
    /// The parts, in order; never empty.
    pub parts: Vec<AssetBundle>,
    /// Removed ids, rooms, icons and entries left out because one alone
    /// doesn't fit a part.
    pub dropped: usize,
}

/// Cuts `bundle` into parts that each encode to at most `max_bytes`, for
/// responses with a size cap. Applied in order with [`fold`], the parts
/// have the effect of `bundle`.
///
/// The first part keeps `bundle`'s `is_delta`, and later parts are deltas.
/// The removed ids come first, then the room, the icon, the emotes, the
/// modifiers and the tags, each in order, so every removal precedes every
/// upsert. A part holds at least its `is_delta` flag, 2 bytes in a delta;
/// an item whose encoding doesn't fit beside that alone is left out and
/// counted in [`Split::dropped`].
pub fn split(bundle: AssetBundle, max_bytes: usize) -> Split {
    let AssetBundle {
        is_delta,
        removed_ids,
        room,
        emotes,
        modifiers,
        tags,
        icon,
    } = bundle;

    let mut s = Splitter {
        parts: Vec::new(),
        len: if is_delta { DELTA_FLAG_BYTES } else { 0 },
        cur: AssetBundle {
            is_delta,
            ..Default::default()
        },
        ids_len: 0,
        max: max_bytes,
        dropped: 0,
    };

    for id in removed_ids {
        s.removed_id(id);
    }
    if let Some(room) = room
        && s.place(field_len(&room))
    {
        s.cur.room = Some(room);
    }
    if let Some(icon) = icon
        && s.place(field_len(&icon))
    {
        s.cur.icon = Some(icon);
    }
    for e in emotes {
        if s.place(field_len(&e)) {
            s.cur.emotes.push(e);
        }
    }
    for e in modifiers {
        if s.place(field_len(&e)) {
            s.cur.modifiers.push(e);
        }
    }
    for e in tags {
        if s.place(field_len(&e)) {
            s.cur.tags.push(e);
        }
    }

    s.parts.push(s.cur);
    Split {
        parts: s.parts,
        dropped: s.dropped,
    }
}

struct Splitter {
    /// The finished parts. Bound: one per item of the input.
    parts: Vec<AssetBundle>,
    cur: AssetBundle,
    /// `cur`'s encoded size.
    len: usize,
    /// The payload size of `cur`'s packed removed ids.
    ids_len: usize,
    max: usize,
    dropped: usize,
}

impl Splitter {
    fn next_part(&mut self) {
        let done = std::mem::replace(
            &mut self.cur,
            AssetBundle {
                is_delta: true,
                ..Default::default()
            },
        );
        self.parts.push(done);
        self.len = DELTA_FLAG_BYTES;
        self.ids_len = 0;
    }

    /// Makes room in `cur` for a field of `len` bytes, starting a new part
    /// when `cur` lacks it, and accounts it. Returns false, counting the
    /// field dropped, when it doesn't fit a new part either.
    fn place(&mut self, len: usize) -> bool {
        if DELTA_FLAG_BYTES.saturating_add(len) > self.max {
            self.dropped += 1;
            return false;
        }
        if self.len.saturating_add(len) > self.max {
            self.next_part();
        }
        self.len += len;
        true
    }

    fn removed_id(&mut self, id: u64) {
        let v = uvarint_len(id);
        let alone = 1 + prost::length_delimiter_len(v) + v;
        if DELTA_FLAG_BYTES + alone > self.max {
            self.dropped += 1;
            return;
        }
        let here = if self.cur.removed_ids.is_empty() {
            alone
        } else {
            v + prost::length_delimiter_len(self.ids_len + v)
                - prost::length_delimiter_len(self.ids_len)
        };
        let added = if self.len.saturating_add(here) > self.max {
            self.next_part();
            alone
        } else {
            here
        };
        self.len += added;
        self.ids_len += v;
        self.cur.removed_ids.push(id);
    }
}

#[cfg(test)]
#[path = "assets_test.rs"]
mod tests;
