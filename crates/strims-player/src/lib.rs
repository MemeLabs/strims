// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The browser players' core: a unit-format stream remuxed into the fMP4
//! a MediaSource (or iOS's ManagedMediaSource) SourceBuffer appends, for
//! the baseline player, or demuxed into the decoder configs and access
//! units WebCodecs decoders take, for the low-latency player.
//!
//! It runs in the UI's player workers, built for wasm32 with wasm-bindgen
//! (`--target web`); the [`Remuxer`] and the [`Demuxer`] are sans-IO, so
//! they build and test natively too. The unit reader and the fMP4 writers
//! are strims-video's ([`strims_video::units::UnitReader`],
//! [`strims_video::fmp4`]), so the wire format has one implementation.
//!
//! - [`remux`]: the [`Remuxer`] and its outputs.
//! - [`demux`]: the [`Demuxer`] and its outputs.
//! - [`mime`]: the SourceBuffer type of an init's tracks.
//! - [`webcodecs`]: the codec strings and descriptions WebCodecs decoders
//!   take, and the durations audio payloads carry.
//! - `wasm` (wasm32 only): the module's API, the `Remuxer` and `Demuxer`
//!   classes that `src/web/strims-player.d.ts` declares.
//!
//! # Input
//!
//! One stream's records as the Egress RPC delivers a
//! `video/x.strims-units` stream: `Data` messages of record bytes, the
//! one completing a record with `segment_end`, and the first after a
//! skip with `discontinuity`, its bytes beginning a whole record
//! ([`strims_video::frontend`]). Any source of the same pieces
//! ([`strims_video::EgressPiece`]) feeds it alike.
//!
//! # Remuxed output
//!
//! The remuxer's outputs ([`Output`]) come in the order a SourceBuffer
//! appends them:
//!
//! - **Inits.** An init segment declares the reader's configs (video
//!   first, then audio, each by track id), and comes at the next keyframe
//!   of the cut track (the lowest-id video track, or audio without video)
//!   after they change, with the keyframe's composition delay in its edit
//!   lists. Units of a track whose config changed are dropped until then,
//!   and units before the first init. It carries the SourceBuffer type and
//!   whether the previous init's SourceBuffer takes it (after
//!   `changeType`); an init no different from the one before isn't output.
//! - **Media.** A fragment (`prft`, `moof`, `mdat`) per unit, as
//!   [`strims_video::fmp4::FragmentWriter`] writes them with a span of
//!   zero: a unit is written once its track's next unit gives its
//!   duration. Each lists the units it holds, with their presentation
//!   times on the SourceBuffer's timeline and their publish times.
//! - **Gaps and discontinuities.** Units lost show as a gap of a track;
//!   a skip of the input as a discontinuity. Neither writes anything out:
//!   each track's unit before a loss lasts until its next unit, so the
//!   picture holds until the next keyframe while audio plays on, and the
//!   buffered timeline has no hole. MediaSource stalls at a hole in any
//!   track, and a jump in one track's decode times makes every track wait
//!   for a keyframe.
//! - **Restarts.** A writer's new session starts its timestamps over. Its
//!   first unit that would decode before its track's last one starts an
//!   epoch: every unit from there on is moved by an offset that puts that
//!   unit at the end of the output so far, and a restart output says so.
//!   Within an epoch, units that still go back are dropped. The output's
//!   decode times never go back, so a SourceBuffer appends across a new
//!   session without `timestampOffset` or a new MediaSource.
//!
//! With these, the output for a stream of one writer session equals what
//! the segmenter writes for the same units with a fragment per unit, each
//! record's media following its init.
//!
//! # Recovery
//!
//! After a fatal media error, the player rebuilds its MediaSource and
//! makes a new [`Remuxer`]: it resyncs at the next GOP's CONFIGS record
//! and starts with an init. The same remuxer goes on across a reopened
//! Egress stream when the new stream's first message is passed as a
//! discontinuity.
//!
//! # Demuxed output
//!
//! The [`Demuxer`] takes the same input and gives, in stream order, the
//! tracks' configs whenever they change, each unit the reader returns
//! with its timestamps, duration, publish time and payload, and the same
//! gaps, discontinuities and restarts. It writes nothing out ahead of a
//! decoder: holding the picture over a loss, A/V sync and the playout
//! clock are the player's. A writer's restart moves the new session to
//! follow the presentation so far, so the output's presentation times
//! never go back across sessions.

pub mod demux;
pub mod mime;
pub mod remux;
#[cfg(target_arch = "wasm32")]
pub mod wasm;
pub mod webcodecs;

pub use demux::{DemuxOutput, DemuxedUnit, Demuxer, DemuxerMetrics};
pub use mime::{mse_codec, mse_type};
pub use remux::{MAX_HELD_BYTES, MAX_HELD_UNITS, Output, Remuxer, RemuxerMetrics, UnitInfo};
