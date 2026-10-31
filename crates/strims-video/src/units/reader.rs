// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The unit-format reader: records in, decoder configs and access units
//! out.

use std::collections::{BTreeMap, VecDeque};

use super::MAX_TRACKS;
use super::record::{ConfigEntries, Record, UnitHeader, parse_record};
use crate::fmp4::check_configs;
use crate::media::{AccessUnit, DecoderConfig, TrackId, TrackKind};

/// What a [`UnitReader::read`] changed, before the unit it returned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReaderEvent {
    /// The media tracks' configs changed ([`UnitReader::configs`]): units
    /// from here on decode with them. It also comes when a track starts
    /// over under a new writer session, whose timestamps start over too.
    Configs,
    /// The text tracks changed ([`UnitReader::text_configs`]): one came,
    /// went, or started over. It comes with [`Configs`](Self::Configs)
    /// when the media tracks changed too.
    TextConfigs,
    /// `missing` units of media track `track` never arrived, by their
    /// sequence numbers. A video track then resumes at its next keyframe.
    /// A text track's missing cues are only counted
    /// ([`UnitReaderMetrics::missing_cues`]).
    Gap { track: TrackId, missing: u64 },
    /// The writer's session ended (an END record). The reader forgot its
    /// tracks ([`UnitReader::configs`] is empty): units come again after
    /// the next CONFIGS record, a later session's, a video track's from
    /// its next keyframe.
    End,
}

/// What a [`UnitReader`] has read and dropped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnitReaderMetrics {
    pub records: u64,
    /// Units returned.
    pub units: u64,
    /// CONFIGS records applied.
    pub configs: u64,
    /// Gaps reported, and the units they counted.
    pub gaps: u64,
    pub missing: u64,
    /// Text tracks' units that never arrived, by their sequence numbers.
    pub missing_cues: u64,
    /// Video units dropped while their track waited for a keyframe.
    pub awaiting_keyframe: u64,
    /// Units dropped for a config version their track doesn't have: a
    /// CONFIGS record was lost.
    pub config_mismatches: u64,
    /// Units of a track without a config.
    pub unknown_tracks: u64,
    /// Units numbered before their track's next: repeated or reordered.
    pub stale: u64,
    /// Units whose decode time is before their track's last one.
    pub out_of_order: u64,
    /// Records refused as malformed, and records of a kind the reader
    /// doesn't know.
    pub malformed: u64,
    pub unknown_kinds: u64,
    /// END records read.
    pub ends: u64,
    /// Entries of a sample entry the reader doesn't know.
    pub unknown_entries: u64,
    /// Losses the caller reported ([`UnitReader::lost`]).
    pub lost: u64,
}

/// A track the reader has a config of.
#[derive(Debug, Clone)]
struct Track {
    version: u32,
    /// The seq of the track's next unit.
    next_seq: u64,
    /// The last unit's decode time returned since the track (re)started.
    last_dts: Option<i64>,
    /// A video track waits for a keyframe.
    awaiting_key: bool,
}

/// Turns unit-format records into decoder configs and access units (see
/// the [format](super)).
///
/// It reads records in stream order, from any record on: units come once
/// it has read a CONFIGS record, a video track's from its next keyframe.
/// Its output is a source's: each track's config comes before its units,
/// and each track's units come in decode order, with non-decreasing DTS.
/// It holds the media tracks' configs ([`configs`](Self::configs)) apart
/// from the text tracks' ([`text_configs`](Self::text_configs)), so a
/// reader of media alone takes a text track's units as those of a track
/// it has no config of.
///
/// Losses show in sequence numbers, which the units and CONFIGS records
/// carry: a gap is a [`ReaderEvent::Gap`], after which a video track
/// resumes at its next keyframe and an audio track at once. A unit of a
/// config version its track doesn't have, from a lost CONFIGS record, is
/// dropped, and its video track then waits for a CONFIGS record of the
/// version and a keyframe. A CONFIGS record whose sequence numbers go
/// back starts the track over. An END record ends the session: the
/// reader forgets its tracks, and reads what follows as a later session.
///
/// # Bounds
///
/// It holds at most [`MAX_TRACKS`] configs and no record bytes. A read's
/// work is linear in the record, but for parsing a changed config.
#[derive(Debug, Default)]
pub struct UnitReader {
    /// Bound: `MAX_TRACKS`.
    tracks: BTreeMap<TrackId, Track>,
    /// The media tracks' configs, in track id order.
    configs: Vec<DecoderConfig>,
    /// The text tracks' configs, in track id order.
    text: Vec<DecoderConfig>,
    /// The last read's events. Bound: a gap per track and two configs
    /// changes, or an end.
    events: VecDeque<ReaderEvent>,
    metrics: UnitReaderMetrics,
}

impl UnitReader {
    pub fn new() -> UnitReader {
        UnitReader::default()
    }

    /// The media tracks' current configs, video and audio, in track id
    /// order.
    pub fn configs(&self) -> &[DecoderConfig] {
        &self.configs
    }

    /// The text tracks' current configs, in track id order.
    pub fn text_configs(&self) -> &[DecoderConfig] {
        &self.text
    }

    pub fn metrics(&self) -> UnitReaderMetrics {
        self.metrics
    }

    /// The next event of the last [`read`](Self::read), which come before
    /// the unit it returned. A read drops the events of the one before.
    pub fn poll_event(&mut self) -> Option<ReaderEvent> {
        self.events.pop_front()
    }

    /// Records may be missing here: a lost frame or chunk, or a stream
    /// discontinuity. The reader counts it; sequence numbers tell what
    /// went missing.
    pub fn lost(&mut self) {
        self.metrics.lost += 1;
    }

    /// Reads the next record: the access unit it carries, if the reader
    /// returns it. A record it can't use is counted and returns nothing.
    pub fn read<'a>(&mut self, record: &'a [u8]) -> Option<AccessUnit<'a>> {
        self.events.clear();
        self.metrics.records += 1;
        match parse_record(record) {
            Ok(Record::Unit {
                header, payload, ..
            }) => self.unit(&header, payload),
            Ok(Record::Configs { entries, .. }) => {
                self.apply_configs(entries);
                None
            }
            Ok(Record::End { .. }) => {
                self.metrics.ends += 1;
                self.tracks.clear();
                self.configs.clear();
                self.text.clear();
                self.events.push_back(ReaderEvent::End);
                None
            }
            Ok(Record::Other { .. }) => {
                self.metrics.unknown_kinds += 1;
                None
            }
            Err(_) => {
                self.metrics.malformed += 1;
                None
            }
        }
    }

    fn gap(&mut self, track: TrackId, missing: u64) {
        if self.text.iter().any(|c| c.track == track) {
            self.metrics.missing_cues = self.metrics.missing_cues.saturating_add(missing);
            return;
        }
        self.metrics.gaps += 1;
        self.metrics.missing = self.metrics.missing.saturating_add(missing);
        self.events.push_back(ReaderEvent::Gap { track, missing });
    }

    fn is_video(&self, track: TrackId) -> bool {
        self.configs
            .iter()
            .any(|c| c.track == track && c.kind == TrackKind::Video)
    }

    fn unit<'a>(&mut self, h: &UnitHeader, payload: &'a [u8]) -> Option<AccessUnit<'a>> {
        let video = self.is_video(h.track);
        let Some(t) = self.tracks.get_mut(&h.track) else {
            self.metrics.unknown_tracks += 1;
            return None;
        };
        if h.seq < t.next_seq {
            self.metrics.stale += 1;
            return None;
        }
        let missing = h.seq - t.next_seq;
        t.next_seq = h.seq.saturating_add(1);
        if missing > 0 && video {
            t.awaiting_key = true;
        }
        let mismatch = h.config != t.version;
        if mismatch && video {
            t.awaiting_key = true;
        }
        let awaiting = t.awaiting_key && !(h.keyframe && !mismatch);
        let backwards = t.last_dts.is_some_and(|last| h.dts < last);
        if backwards && video {
            t.awaiting_key = true;
        }
        let deliver = !mismatch && !awaiting && !backwards;
        if deliver {
            t.awaiting_key = false;
            t.last_dts = Some(h.dts);
        }
        if missing > 0 {
            self.gap(h.track, missing);
        }
        if mismatch {
            self.metrics.config_mismatches += 1;
        } else if awaiting {
            self.metrics.awaiting_keyframe += 1;
        } else if backwards {
            self.metrics.out_of_order += 1;
        }
        if !deliver {
            return None;
        }
        self.metrics.units += 1;
        Some(AccessUnit {
            track: h.track,
            dts: h.dts,
            pts: h.pts,
            keyframe: h.keyframe,
            payload,
            published: h.published,
        })
    }

    /// Applies a CONFIGS record: the tracks it lists are the stream's, with
    /// their configs and next sequence numbers.
    fn apply_configs(&mut self, entries: ConfigEntries<'_>) {
        // Configs as listed, each with its entry's version and next seq, and
        // whether it is the one the reader holds.
        let mut next: Vec<(DecoderConfig, u32, u64, bool)> = Vec::with_capacity(MAX_TRACKS);
        for e in entries {
            if !e.is_known() {
                self.metrics.unknown_entries += 1;
                continue;
            }
            let held = self
                .configs
                .iter()
                .chain(&self.text)
                .find(|c| c.track == e.track)
                .filter(|c| e.describes(c));
            let config = match held {
                Some(c) => c.clone(),
                None => match e.decoder_config() {
                    Some(Ok(c)) => c,
                    Some(Err(_)) | None => {
                        self.metrics.malformed += 1;
                        return;
                    }
                },
            };
            next.push((config, e.version, e.next_seq, held.is_some()));
        }
        next.sort_by_key(|(c, ..)| c.track);
        let configs: Vec<DecoderConfig> = next.iter().map(|(c, ..)| c.clone()).collect();
        if check_configs(&configs).is_err() {
            self.metrics.malformed += 1;
            return;
        }
        self.metrics.configs += 1;
        let (text, configs): (Vec<DecoderConfig>, Vec<DecoderConfig>) =
            configs.into_iter().partition(|c| c.kind == TrackKind::Text);
        let mut changed = configs.len() != self.configs.len();
        let mut text_changed = text.len() != self.text.len();
        let mut tracks = BTreeMap::new();
        for (config, version, next_seq, same) in next {
            let video = config.kind == TrackKind::Video;
            let is_text = config.kind == TrackKind::Text;
            // A text track's units say nothing of how to decode media.
            let changed = if is_text {
                &mut text_changed
            } else {
                &mut changed
            };
            let track = match self.tracks.remove(&config.track) {
                Some(mut t) => {
                    let restarted = next_seq < t.next_seq;
                    if next_seq > t.next_seq {
                        self.gap(config.track, next_seq - t.next_seq);
                        t.awaiting_key |= video;
                    }
                    if restarted {
                        t.last_dts = None;
                        t.awaiting_key = video;
                    }
                    if !same || version != t.version {
                        t.awaiting_key |= video;
                    }
                    *changed |= restarted || !same || version != t.version;
                    t.version = version;
                    t.next_seq = next_seq;
                    t
                }
                None => {
                    *changed = true;
                    Track {
                        version,
                        next_seq,
                        last_dts: None,
                        awaiting_key: video,
                    }
                }
            };
            tracks.insert(config.track, track);
        }
        self.tracks = tracks;
        if changed {
            self.configs = configs;
            self.events.push_back(ReaderEvent::Configs);
        }
        if text_changed {
            self.text = text;
            self.events.push_back(ReaderEvent::TextConfigs);
        }
    }
}

#[cfg(test)]
#[path = "reader_test.rs"]
mod tests;
