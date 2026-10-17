// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Bounded decoding of a chat server's event and asset records for a
//! client. Sans-IO: the caller reads the records of each stream (with
//! `strims_transfer::RecordReader`) and feeds each payload to its reader,
//! telling the reader about each discontinuity.
//!
//! The server is a peer: its records may be malformed or hostile.
//! [`RoomLimits`] bounds what a record makes the client decode and hold,
//! and [`ReaderMetrics`] counts what was dropped.
//!
//! - **Events.** [`EventReader`] yields each record that decodes as a
//!   `ServerEvent` carrying a message within the bounds. A message with
//!   more entities than the bound is dropped before it's decoded.
//! - **Assets.** [`AssetReader`] yields each `AssetBundle`, decoded entry
//!   by entry: entries past a count bound, or whose nested fields would
//!   take the bundle past its node bound, are left out. After a start or
//!   discontinuity it drops deltas until a unified bundle arrives, since a
//!   delta applies only on top of the records before it.
//! - **Partial records.** After a start or discontinuity, the first record
//!   may begin inside an earlier one, so a first record that doesn't decode
//!   is counted apart from other malformed records.

use prost::Message;
use strims_chunkstream::get_uvarint;
use strims_proto::chat::v1::server_event::Body;
use strims_proto::chat::v1::{AssetBundle, ServerEvent};

use crate::entities::MAX_ENTITIES;

/// The largest event record, in bytes.
pub const MAX_EVENT_RECORD: usize = 64 * 1024;

/// The largest asset record, in bytes: the asset stream's window.
pub const MAX_ASSET_RECORD: usize = crate::assets::ASSET_WINDOW_BYTES;

/// What a client decodes and holds of one room's streams.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoomLimits {
    /// The largest event record, in bytes: the `max_record` of the event
    /// stream's record reader.
    pub max_event_record: usize,
    /// Entities of one message, counting every link, emote, emoji, nick,
    /// tag, code block and spoiler and the greentext and `/me` spans. A
    /// message with more is dropped.
    pub max_entities: usize,
    /// Message body bytes. A message with a longer body is dropped.
    pub max_body_bytes: usize,
    /// The largest asset record, in bytes: the `max_record` of the asset
    /// stream's record reader. Also the encoded size of a bundle
    /// [`fold`](crate::assets::fold) accumulates.
    pub max_asset_record: usize,
    /// Emotes, modifiers and tags of one bundle; the first are kept.
    pub max_emotes: usize,
    pub max_modifiers: usize,
    pub max_tags: usize,
    /// Removed ids of one bundle; the first are kept.
    pub max_removed_ids: usize,
    /// Length-delimited fields (strings, bytes and messages, at any depth)
    /// across the emotes, modifiers and tags of one bundle. Each costs an
    /// allocation when decoded; an entry that would take the bundle past
    /// the bound is left out.
    pub max_bundle_nodes: usize,
}

impl Default for RoomLimits {
    fn default() -> Self {
        RoomLimits {
            max_event_record: MAX_EVENT_RECORD,
            max_entities: MAX_ENTITIES,
            max_body_bytes: 8 * 1024,
            max_asset_record: MAX_ASSET_RECORD,
            max_emotes: 8192,
            max_modifiers: 1024,
            max_tags: 1024,
            max_removed_ids: 8192 + 1024 + 1024,
            max_bundle_nodes: 256 * 1024,
        }
    }
}

/// What a reader yielded and dropped.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReaderMetrics {
    /// Events and bundles yielded.
    pub events: u64,
    pub bundles: u64,
    /// First records after a start or discontinuity that didn't decode.
    pub partial_records: u64,
    /// Other records that didn't decode.
    pub malformed_records: u64,
    /// Records over their size bound.
    pub oversized_records: u64,
    /// Events without a message.
    pub empty_events: u64,
    /// Messages dropped for their body's size or their entity count.
    pub long_bodies: u64,
    pub entity_overflows: u64,
    /// Deltas dropped while waiting for a unified bundle.
    pub deltas_dropped: u64,
    /// Bundle entries and removed ids left out for a bound.
    pub emotes_dropped: u64,
    pub modifiers_dropped: u64,
    pub tags_dropped: u64,
    pub removed_ids_dropped: u64,
}

/// Decodes the records of a room's event stream.
#[derive(Clone, Debug)]
pub struct EventReader {
    limits: RoomLimits,
    /// The next record is the first after a start or discontinuity.
    first: bool,
    metrics: ReaderMetrics,
}

impl EventReader {
    pub fn new(limits: RoomLimits) -> Self {
        EventReader {
            limits,
            first: true,
            metrics: ReaderMetrics::default(),
        }
    }

    /// The event of one record, or `None` when the record is dropped.
    pub fn handle_record(&mut self, record: &[u8]) -> Option<ServerEvent> {
        let first = std::mem::replace(&mut self.first, false);
        if record.len() > self.limits.max_event_record {
            self.metrics.oversized_records += 1;
            return None;
        }
        let entities = match entity_count(record) {
            Ok(n) => n,
            Err(Malformed) => {
                self.malformed(first);
                return None;
            }
        };
        if entities > self.limits.max_entities {
            self.metrics.entity_overflows += 1;
            return None;
        }
        let Ok(event) = ServerEvent::decode(record) else {
            self.malformed(first);
            return None;
        };
        match &event.body {
            None => {
                self.metrics.empty_events += 1;
                None
            }
            Some(Body::Message(m)) if m.body.len() > self.limits.max_body_bytes => {
                self.metrics.long_bodies += 1;
                None
            }
            Some(Body::Message(_)) => {
                self.metrics.events += 1;
                Some(event)
            }
        }
    }

    /// The stream started over or skipped: the next record may begin
    /// inside an earlier one.
    pub fn discontinuity(&mut self) {
        self.first = true;
    }

    pub fn metrics(&self) -> &ReaderMetrics {
        &self.metrics
    }

    fn malformed(&mut self, first: bool) {
        if first {
            self.metrics.partial_records += 1;
        } else {
            self.metrics.malformed_records += 1;
        }
    }
}

/// Decodes the records of a room's asset stream.
#[derive(Clone, Debug)]
pub struct AssetReader {
    limits: RoomLimits,
    /// The next record is the first after a start or discontinuity.
    first: bool,
    /// A unified bundle arrived since the last start or discontinuity.
    synced: bool,
    metrics: ReaderMetrics,
}

impl AssetReader {
    pub fn new(limits: RoomLimits) -> Self {
        AssetReader {
            limits,
            first: true,
            synced: false,
            metrics: ReaderMetrics::default(),
        }
    }

    /// The bundle of one record, or `None` when the record is dropped.
    /// Until a unified bundle arrives after a start or discontinuity,
    /// deltas are dropped.
    pub fn handle_record(&mut self, record: &[u8]) -> Option<AssetBundle> {
        let first = std::mem::replace(&mut self.first, false);
        if record.len() > self.limits.max_asset_record {
            self.metrics.oversized_records += 1;
            return None;
        }
        let Ok(bundle) = decode_bundle(record, &self.limits, &mut self.metrics) else {
            if first {
                self.metrics.partial_records += 1;
            } else {
                self.metrics.malformed_records += 1;
            }
            return None;
        };
        if bundle.is_delta && !self.synced {
            self.metrics.deltas_dropped += 1;
            return None;
        }
        self.synced = true;
        self.metrics.bundles += 1;
        Some(bundle)
    }

    /// The stream started over or skipped: deltas are dropped until the
    /// next unified bundle.
    pub fn discontinuity(&mut self) {
        self.first = true;
        self.synced = false;
    }

    pub fn metrics(&self) -> &ReaderMetrics {
        &self.metrics
    }
}

/// Malformed protobuf wire data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Malformed;

#[derive(Clone, Copy, Debug)]
enum Value<'a> {
    Varint(u64),
    Fixed,
    Bytes(&'a [u8]),
}

/// The fields of an encoded message, in order. Groups are malformed, and
/// the iteration ends after the first malformed field.
struct Fields<'a> {
    buf: &'a [u8],
}

impl<'a> Fields<'a> {
    fn new(buf: &'a [u8]) -> Self {
        Fields { buf }
    }

    fn varint(&mut self) -> Result<u64, Malformed> {
        let (v, n) = get_uvarint(self.buf).ok_or(Malformed)?;
        self.buf = self.buf.get(n..).ok_or(Malformed)?;
        Ok(v)
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8], Malformed> {
        let (v, rest) = self.buf.split_at_checked(n).ok_or(Malformed)?;
        self.buf = rest;
        Ok(v)
    }

    fn field(&mut self) -> Result<(u32, Value<'a>), Malformed> {
        let key = u32::try_from(self.varint()?).map_err(|_| Malformed)?;
        let number = key >> 3;
        if number == 0 {
            return Err(Malformed);
        }
        let value = match key & 7 {
            0 => Value::Varint(self.varint()?),
            1 => self.take(8).map(|_| Value::Fixed)?,
            2 => {
                let n = usize::try_from(self.varint()?).map_err(|_| Malformed)?;
                Value::Bytes(self.take(n)?)
            }
            5 => self.take(4).map(|_| Value::Fixed)?,
            _ => return Err(Malformed),
        };
        Ok((number, value))
    }
}

impl<'a> Iterator for Fields<'a> {
    type Item = Result<(u32, Value<'a>), Malformed>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.buf.is_empty() {
            return None;
        }
        let r = self.field();
        if r.is_err() {
            self.buf = &[];
        }
        Some(r)
    }
}

/// The entities of the message in an encoded `ServerEvent`, counting each
/// occurrence of an `Entities` field, across the occurrences of the
/// message and its entities that a decode merges.
fn entity_count(record: &[u8]) -> Result<usize, Malformed> {
    let mut n = 0usize;
    for f in Fields::new(record) {
        let (1001, Value::Bytes(message)) = f? else {
            continue;
        };
        for f in Fields::new(message) {
            let (5, Value::Bytes(entities)) = f? else {
                continue;
            };
            for f in Fields::new(entities) {
                if let (1..=9, _) = f? {
                    n += 1;
                }
            }
        }
    }
    Ok(n)
}

/// The message-typed fields of a message type, by field number.
struct Shape(&'static [(u32, &'static Shape)]);

/// A message without message-typed fields.
const LEAF: Shape = Shape(&[]);
/// `StyleSheet.Asset`: its image.
const STYLE_SHEET_ASSET: Shape = Shape(&[(2, &LEAF)]);
/// `StyleSheet`: its assets.
const STYLE_SHEET: Shape = Shape(&[(3, &STYLE_SHEET_ASSET)]);
/// `EmoteEffect.CustomCSS`: its style sheet.
const CUSTOM_CSS: Shape = Shape(&[(1, &STYLE_SHEET)]);
/// `EmoteEffect`: custom CSS, sprite animation and default modifiers.
const EMOTE_EFFECT: Shape = Shape(&[(1001, &CUSTOM_CSS), (1002, &LEAF), (1003, &LEAF)]);
/// `Emote`: images, effects and contributor.
const EMOTE: Shape = Shape(&[(4, &LEAF), (5, &EMOTE_EFFECT), (6, &LEAF)]);
/// `Modifier`: its style sheet.
const MODIFIER: Shape = Shape(&[(8, &STYLE_SHEET)]);
/// `Tag`.
const TAG: Shape = LEAF;

/// Why [`count_nodes`] stopped.
enum Rejected {
    Malformed,
    Budget,
}

/// Takes one from `budget` for each length-delimited field of `buf`, and
/// of the message-typed fields `shape` names, recursively. The recursion
/// is as deep as the shapes nest.
fn count_nodes(buf: &[u8], shape: &Shape, budget: &mut usize) -> Result<(), Rejected> {
    for f in Fields::new(buf) {
        let (number, value) = f.map_err(|Malformed| Rejected::Malformed)?;
        if let Value::Bytes(b) = value {
            *budget = budget.checked_sub(1).ok_or(Rejected::Budget)?;
            if let Some((_, sub)) = shape.0.iter().find(|(n, _)| *n == number) {
                count_nodes(b, sub, budget)?;
            }
        }
    }
    Ok(())
}

/// Decodes an `AssetBundle` within `limits`, counting what's left out.
///
/// The room and icon merge as a decode merges them: they hold no repeated
/// fields, so their size is bounded by the record's.
fn decode_bundle(
    record: &[u8],
    limits: &RoomLimits,
    metrics: &mut ReaderMetrics,
) -> Result<AssetBundle, Malformed> {
    let mut b = AssetBundle::default();
    let mut budget = limits.max_bundle_nodes;
    let mut dropped = ReaderMetrics::default();

    for f in Fields::new(record) {
        match f? {
            (1, Value::Varint(v)) => b.is_delta = v != 0,
            (2, Value::Varint(id)) => push_id(&mut b.removed_ids, id, limits, &mut dropped),
            (2, Value::Bytes(packed)) => {
                let mut ids = packed;
                while !ids.is_empty() {
                    let (id, n) = get_uvarint(ids).ok_or(Malformed)?;
                    ids = ids.get(n..).ok_or(Malformed)?;
                    push_id(&mut b.removed_ids, id, limits, &mut dropped);
                }
            }
            (3, Value::Bytes(room)) => b
                .room
                .get_or_insert_default()
                .merge(room)
                .map_err(|_| Malformed)?,
            (4, Value::Bytes(e)) => entry(
                &mut b.emotes,
                e,
                &EMOTE,
                limits.max_emotes,
                &mut budget,
                &mut dropped.emotes_dropped,
            )?,
            (5, Value::Bytes(e)) => entry(
                &mut b.modifiers,
                e,
                &MODIFIER,
                limits.max_modifiers,
                &mut budget,
                &mut dropped.modifiers_dropped,
            )?,
            (6, Value::Bytes(e)) => entry(
                &mut b.tags,
                e,
                &TAG,
                limits.max_tags,
                &mut budget,
                &mut dropped.tags_dropped,
            )?,
            (7, Value::Bytes(icon)) => b
                .icon
                .get_or_insert_default()
                .merge(icon)
                .map_err(|_| Malformed)?,
            (1..=7, _) => return Err(Malformed),
            _ => {}
        }
    }

    metrics.removed_ids_dropped += dropped.removed_ids_dropped;
    metrics.emotes_dropped += dropped.emotes_dropped;
    metrics.modifiers_dropped += dropped.modifiers_dropped;
    metrics.tags_dropped += dropped.tags_dropped;
    Ok(b)
}

fn push_id(ids: &mut Vec<u64>, id: u64, limits: &RoomLimits, dropped: &mut ReaderMetrics) {
    if ids.len() < limits.max_removed_ids {
        ids.push(id);
    } else {
        dropped.removed_ids_dropped += 1;
    }
}

/// Decodes one entry into `out`, or leaves it out and counts it when `out`
/// is full or its fields would exhaust `budget`.
fn entry<T: Message + Default>(
    out: &mut Vec<T>,
    buf: &[u8],
    shape: &Shape,
    max: usize,
    budget: &mut usize,
    dropped: &mut u64,
) -> Result<(), Malformed> {
    if out.len() >= max {
        *dropped += 1;
        return Ok(());
    }
    let mut left = budget.saturating_sub(1);
    match count_nodes(buf, shape, &mut left) {
        Ok(()) if *budget > 0 => {
            *budget = left;
            out.push(T::decode(buf).map_err(|_| Malformed)?);
        }
        Ok(()) | Err(Rejected::Budget) => *dropped += 1,
        Err(Rejected::Malformed) => return Err(Malformed),
    }
    Ok(())
}

#[cfg(test)]
#[path = "reader_test.rs"]
mod tests;
