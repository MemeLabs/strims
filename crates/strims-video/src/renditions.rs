// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A session's renditions: the swarms it writes besides its channel's,
//! and the transcode that makes them ([`ingress`](super#renditions)).

use std::cell::Cell;
use std::collections::{BTreeSet, VecDeque};
use std::rc::Rc;
use std::task::{Context, Poll};

use hmac::{Hmac, KeyInit, Mac};
use sha2::Sha256;
use strims_base::time::{Duration, Time};
use strims_directory::Host;
use strims_kv::Kv;
use strims_ppspp::{SwarmId, SwarmOptions, Uri, WriterOptions};
use strims_proto::network::v1::directory::ListingRendition;
use strims_proto::types::Key;
use strims_rpc::Timer;
use strims_transfer::{SwarmHost, SwarmWrite, TransferId};

use std::future::poll_fn;
use std::pin::pin;

use futures_util::future::{Either, select};

use super::{End, IngestSwarm, Live, REPLACED, Shared, Sign, unit_ingress_options, untranscoded};
use crate::captions::{CaptionCarry, CcTuple, NalFormat};
use crate::codec::{
    Codec, CodecEvent, Input, Output, Refused, RenditionPlan, Transcode, TranscodePlan,
};
use crate::config::{TranscodeMetrics, TranscodeSettings};
use crate::host::VideoHost;
use crate::media::{AccessUnit, CodecParams, DecoderConfig, Muxer, TrackId, TrackKind, units_mime};
use crate::registry::SwarmKind;
use crate::units::{SignTimer, UnitSigning, UnitWriter, UnitWriterMetrics};

/// The label a rendition key's derivation starts its message with.
pub(super) const KEY_LABEL: &[u8] = b"strims-video-rendition:";
/// The id of the rendition of a stream's own encoding, when another is
/// its channel's.
pub(super) const SOURCE_ID: &str = "source";
/// The audio bitrate a rendition's nominal bitrate counts when the stream
/// states none.
const AUDIO_BPS: u64 = 128_000;

/// The key of a channel's rendition `id`: the ed25519 key whose seed is
/// the HMAC-SHA256, keyed with the channel key's private half, of
/// [`KEY_LABEL`] followed by `id`. A channel's rendition of an id keeps
/// its swarm id from one session to the next, so a session that replaces
/// another takes its renditions' slots, and their streams go on.
pub(super) fn rendition_key(channel: &Key, id: &str) -> Key {
    // invariant: HMAC takes a key of any length.
    #[allow(clippy::expect_used)]
    let mut mac = Hmac::<Sha256>::new_from_slice(&channel.private).expect("any key length");
    mac.update(KEY_LABEL);
    mac.update(id.as_bytes());
    let seed: [u8; 32] = mac.finalize().into_bytes().into();
    strims_crypto::key_from_seed(&seed)
}

/// The id of a rendition of `plan`: its shorter side and frame rate cap,
/// such as `480p30`.
pub(super) fn rung_id(plan: &RenditionPlan) -> String {
    format!("{}p{}", plan.short_side, plan.max_frame_rate)
}

/// What a session makes of its stream, given its video config.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Planned {
    /// The transcode, if any: the H.264 of a stream in another codec
    /// first, then each rung.
    pub(super) transcode: Option<TranscodePlan>,
    /// The stream's channel swarm carries the transcode's first
    /// rendition: the stream isn't H.264.
    pub(super) main_transcoded: bool,
    /// The rendition swarms, by id: the stream's own encoding
    /// ([`SOURCE_ID`]) when it isn't H.264, then the rungs.
    pub(super) outputs: Vec<(String, Option<RenditionPlan>)>,
}

/// The renditions `settings` make of a stream whose video config is
/// `video`; see [`ingress`](super#renditions).
pub(super) fn plan(video: &DecoderConfig, settings: &TranscodeSettings) -> Planned {
    let (w, h) = video.params.picture_size().unwrap_or((0, 0));
    let short = w.min(h);
    let h264 = matches!(video.params, CodecParams::Avc { .. });
    let mut renditions = Vec::new();
    let mut outputs = Vec::new();
    let mut top = short;
    if !h264 {
        let cap = settings.h264_bitrates.last().map_or(short, |&(s, _)| s);
        let side = short.min(cap);
        let bitrate = settings
            .h264_bitrates
            .iter()
            .find(|&&(s, _)| s >= side)
            .or(settings.h264_bitrates.last())
            .map_or(0, |&(_, b)| b);
        renditions.push(RenditionPlan {
            short_side: side,
            max_frame_rate: settings.h264_max_frame_rate,
            bitrate,
        });
        outputs.push((SOURCE_ID.to_owned(), None));
        top = side;
    }
    let max = crate::codec::MAX_RENDITIONS.saturating_sub(renditions.len());
    for rung in settings
        .ladder
        .iter()
        .filter(|r| r.short_side < top)
        .take(max)
    {
        renditions.push(*rung);
        outputs.push((rung_id(rung), Some(*rung)));
    }
    Planned {
        transcode: (!renditions.is_empty()).then_some(TranscodePlan { renditions }),
        main_transcoded: !h264,
        outputs,
    }
}

/// One of the transcodes the ingress runs at once; dropping it frees its
/// place ([`TranscodeSettings::max_sessions`]).
pub(super) struct TranscodePlace(Rc<Cell<usize>>);

impl TranscodePlace {
    /// A place among `running`'s, if fewer than `max` run.
    pub(super) fn take(running: &Rc<Cell<usize>>, max: usize) -> Option<TranscodePlace> {
        (running.get() < max).then(|| {
            running.set(running.get() + 1);
            TranscodePlace(running.clone())
        })
    }
}

impl Drop for TranscodePlace {
    fn drop(&mut self) {
        self.0.set(self.0.get().saturating_sub(1));
    }
}

/// A rendition swarm of a session: its writer, signing and swarm.
pub(super) struct RenditionOutput<W> {
    pub(super) id: String,
    key: Key,
    pub(super) swarm_id: SwarmId,
    pub(super) t: TransferId,
    signing: UnitSigning,
    pub(super) writer: UnitWriter,
    timer: SignTimer,
    /// Once created; `None` with `created` set when creating it failed.
    pub(super) swarm: Option<IngestSwarm<W>>,
    created: bool,
    pub(super) slot: Option<u64>,
    counted: UnitWriterMetrics,
    /// Its nominal bitrate and frame rate, for its listing.
    bitrate: u64,
    frame_rate: f64,
    /// What the snippet lists of it, once it wrote a record.
    pub(super) listed: Option<ListingRendition>,
}

impl<W: SwarmWrite> RenditionOutput<W> {
    fn new(channel: &Key, id: &str, bitrate: u64, frame_rate: f64, frame_size: usize) -> Self {
        let key = rendition_key(channel, id);
        let swarm_id = SwarmId::new(&key.public);
        let signing = UnitSigning::for_stream(Some(bitrate), false);
        // invariant: the ingest chunk size is a valid frame size.
        #[allow(clippy::expect_used)]
        let writer = UnitWriter::new(frame_size).expect("the ingest chunk size is a frame size");
        RenditionOutput {
            id: id.to_owned(),
            t: TransferId::new(&swarm_id, b""),
            swarm_id,
            key,
            timer: signing.timer(frame_size),
            signing,
            writer,
            swarm: None,
            created: false,
            slot: None,
            counted: UnitWriterMetrics::default(),
            bitrate,
            frame_rate,
            listed: None,
        }
    }

    fn options(&self) -> SwarmOptions {
        unit_ingress_options(&self.signing)
    }

    /// Takes an event of its stream; refusals drop it.
    fn take(&mut self, output: &Output) {
        let refused = match output {
            Output::Config(c) => self.writer.set_config(c),
            Output::Unit(u) => self.writer.push(&u.as_access_unit()),
        };
        if let Err(e) = refused {
            tracing::debug!(rendition = %self.id, error = %e, "a rendition's writer refused an event");
        }
    }

    /// Takes a unit of its stream; refusals drop it.
    fn take_unit(&mut self, unit: &AccessUnit<'_>) {
        if let Err(e) = self.writer.push(unit) {
            tracing::debug!(rendition = %self.id, error = %e, "a rendition's writer refused a unit");
        }
    }

    /// Writes what the writer holds into the swarm, once it exists.
    pub(super) async fn write(&mut self, now: Time) -> Result<(), String> {
        let Some(swarm) = &mut self.swarm else {
            return Ok(());
        };
        let out = self.writer.take_output();
        let res = if out.is_empty() {
            Ok(())
        } else {
            swarm.writer.write(&out).await.map_err(|e| e.to_string())
        };
        self.writer.restore(out);
        if res.is_ok() {
            self.timer.written(self.writer.offset(), now);
        }
        res
    }

    /// Has the swarm sign its partial integrity unit; see
    /// [`ingress`](super#media).
    pub(super) async fn sign(&mut self, now: Time) -> Result<(), String> {
        self.writer.end_frame();
        self.write(now).await?;
        let Some(swarm) = &mut self.swarm else {
            return Ok(());
        };
        let next = swarm.writer.flush().await.map_err(|e| e.to_string())?;
        self.writer.continue_at(next).map_err(|e| e.to_string())?;
        self.timer.flushed(next);
        Ok(())
    }

    pub(super) fn sign_due(&self) -> Option<Time> {
        self.swarm.as_ref().and(self.timer.due())
    }

    fn partial(&self) -> bool {
        self.swarm.is_some() && self.timer.is_partial()
    }

    /// Records and bytes written since the last count, and its listing
    /// once it wrote a record.
    fn take_records(&mut self) -> (u64, u64, bool) {
        let m = self.writer.metrics();
        let records = m.records.saturating_sub(self.counted.records);
        let bytes = m.bytes.saturating_sub(self.counted.bytes);
        self.counted = m;
        let Some(swarm) = &self.swarm else {
            return (records, bytes, false);
        };
        // A rendition that ended is listed no more.
        if m.ends > 0 {
            let changed = self.listed.take().is_some();
            return (records, bytes, changed);
        }
        if records == 0 {
            return (0, bytes, false);
        }
        let listed = Some(listing(
            self.writer.configs(),
            &swarm.uri,
            self.bitrate,
            self.frame_rate,
        ));
        let changed = listed != self.listed;
        self.listed = listed;
        (records, bytes, changed)
    }
}

/// The snippet's entry of a rendition whose writer has `configs`.
pub(super) fn listing(
    configs: &[DecoderConfig],
    uri: &str,
    bitrate: u64,
    frame_rate: f64,
) -> ListingRendition {
    let (width, height) = configs
        .iter()
        .find(|c| c.kind == TrackKind::Video)
        .and_then(|c| c.params.picture_size())
        .unwrap_or((0, 0));
    ListingRendition {
        mime_type: units_mime(configs),
        swarm_uri: uri.to_owned(),
        bitrate,
        width: u32::from(width),
        height: u32::from(height),
        frame_rate,
    }
}

/// Where a transcode's rendition goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Target {
    /// The channel's swarm.
    Main,
    Output(usize),
}

/// A session's renditions and their transcode.
pub(super) struct Renditions<C: Codec, W> {
    codec: C,
    settings: TranscodeSettings,
    plan: Option<TranscodePlan>,
    /// Where each of the plan's renditions goes.
    targets: Vec<Target>,
    pub(super) outputs: Vec<RenditionOutput<W>>,
    /// The output of the stream's own encoding, when another is its
    /// channel's.
    passthrough: Option<usize>,
    pub(super) transcode: Option<C::Transcode>,
    _place: Option<TranscodePlace>,
    /// The stream's configs, which a new transcode takes first.
    configs: Vec<DecoderConfig>,
    /// Input waits for the stream's next video keyframe.
    await_keyframe: bool,
    /// Since when the transcode has held input without output.
    stalled_since: Option<Time>,
    /// When the transcode started, its restarts among them. Bound:
    /// `max_restarts` + 1.
    starts: VecDeque<Time>,
    /// The renditions were given up.
    pub(super) abandoned: bool,
    /// The stream's video track, its timescale and its units' NAL format,
    /// which its captions are read in.
    source_video: Option<(TrackId, u32, NalFormat)>,
    /// The stream's captions, carried to the transcode's renditions, and
    /// each rendition's video track, timescale and NAL format, once its
    /// config came.
    captions: CaptionCarry,
    output_video: Vec<Option<(TrackId, u32, NalFormat)>>,
    /// Reused for a unit's caption tuples.
    tuples: Vec<CcTuple>,
}

/// What a session's renditions report to it.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Report {
    /// Nothing for the session.
    None,
    /// The transcode was given up: the channel's swarm, if it carried a
    /// rendition, has nothing more to write.
    Abandoned(String),
}

impl<C: Codec, W: SwarmWrite> Renditions<C, W> {
    /// The renditions of a stream of channel key `channel` whose video
    /// config is `video`, if `planned` makes any.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        codec: C,
        settings: &TranscodeSettings,
        planned: &Planned,
        place: Option<TranscodePlace>,
        channel: &Key,
        nominal: Option<u64>,
        frame_rate: f64,
        frame_size: usize,
    ) -> Self {
        let mut targets = Vec::new();
        let mut outputs = Vec::new();
        let mut passthrough = None;
        if planned.main_transcoded {
            targets.push(Target::Main);
        }
        for (id, rung) in &planned.outputs {
            match rung {
                None => {
                    passthrough = Some(outputs.len());
                    let bitrate = nominal.unwrap_or(0);
                    outputs.push(RenditionOutput::new(
                        channel, id, bitrate, frame_rate, frame_size,
                    ));
                }
                Some(r) => {
                    targets.push(Target::Output(outputs.len()));
                    let rate = if frame_rate > 0.0 {
                        frame_rate.min(f64::from(r.max_frame_rate))
                    } else {
                        0.0
                    };
                    let bitrate = r.bitrate.saturating_add(AUDIO_BPS);
                    outputs.push(RenditionOutput::new(channel, id, bitrate, rate, frame_size));
                }
            }
        }
        let made = planned.transcode.as_ref().map_or(0, |p| p.renditions.len());
        Renditions {
            codec,
            settings: settings.clone(),
            plan: planned.transcode.clone(),
            targets,
            outputs,
            passthrough,
            transcode: None,
            _place: place,
            configs: Vec::new(),
            await_keyframe: true,
            stalled_since: None,
            starts: VecDeque::new(),
            abandoned: false,
            source_video: None,
            captions: CaptionCarry::new(made),
            output_video: vec![None; made],
            tuples: Vec::new(),
        }
    }

    /// Takes a config of the stream.
    pub(super) fn config(&mut self, c: &DecoderConfig) {
        if c.kind == TrackKind::Video {
            self.source_video = NalFormat::of(&c.params).map(|f| (c.track, c.timescale, f));
        }
        match self.configs.iter_mut().find(|o| o.track == c.track) {
            Some(o) => *o = c.clone(),
            None => self.configs.push(c.clone()),
        }
        if let Some(i) = self.passthrough
            && let Some(o) = self.outputs.get_mut(i)
        {
            o.take(&Output::Config(c.clone()));
        }
        if let Some(t) = &mut self.transcode {
            let _ = t.send(Input::Config(c));
        }
    }

    /// Takes a unit of the stream at `now`, starting the transcode at its
    /// first video keyframe and again after a stall.
    pub(super) fn unit(&mut self, u: &AccessUnit<'_>, now: Time, m: &mut TranscodeMetrics) {
        if let Some(i) = self.passthrough
            && let Some(o) = self.outputs.get_mut(i)
        {
            o.take_unit(u);
        }
        let video = self
            .configs
            .iter()
            .any(|c| c.track == u.track && c.kind == TrackKind::Video);
        if self.abandoned || self.plan.is_none() {
            return;
        }
        if self.await_keyframe {
            if !(video && u.keyframe) {
                return;
            }
            if self.transcode.is_none() && !self.start(now, m) {
                return;
            }
            if let Some(t) = &mut self.transcode {
                for c in &self.configs {
                    let _ = t.send(Input::Config(c));
                }
            }
            self.await_keyframe = false;
        }
        let Some(t) = &mut self.transcode else {
            return;
        };
        match t.send(Input::Unit(u)) {
            Ok(()) => {
                self.stalled_since.get_or_insert(now);
                if let Some((track, timescale, nals)) = self.source_video
                    && track == u.track
                {
                    self.tuples.clear();
                    nals.cc_tuples(u.payload, &mut self.tuples);
                    self.captions.source(u.pts, timescale, &self.tuples);
                }
            }
            Err(Refused::Full) => {
                m.dropped_gops += 1;
                self.await_keyframe = true;
            }
            Err(Refused::Media(_)) => {}
        }
    }

    /// Starts the transcode, if its restarts allow: whether it runs.
    fn start(&mut self, now: Time, m: &mut TranscodeMetrics) -> bool {
        let Some(plan) = &self.plan else {
            return false;
        };
        while self
            .starts
            .front()
            .is_some_and(|&t| now - t > self.settings.restart_window)
        {
            self.starts.pop_front();
        }
        if self.starts.len() > self.settings.max_restarts {
            return false;
        }
        if !self.starts.is_empty() {
            m.restarts += 1;
        }
        self.starts.push_back(now);
        match self.codec.transcode(plan) {
            Ok(t) => {
                m.starts += 1;
                self.transcode = Some(t);
                self.stalled_since = None;
                self.captions.restart();
                true
            }
            Err(e) => {
                m.failures += 1;
                tracing::warn!(error = %e, "a transcode didn't start");
                false
            }
        }
    }

    /// Polls the transcode for its next event: where it goes, and what.
    pub(super) fn poll_event(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<CodecEvent, String>>> {
        match &mut self.transcode {
            Some(t) => t
                .poll_next(cx)
                .map(|e| e.map(|e| e.map_err(|e| e.to_string()))),
            None => Poll::Pending,
        }
    }

    /// Takes the transcode's `event` at `now`, with the stream's captions
    /// up to its time if it is a video unit: an event for the channel's
    /// swarm, if it is one.
    pub(super) fn event(&mut self, mut event: CodecEvent, now: Time) -> Option<Output> {
        self.carry_captions(&mut event);
        self.stalled_since = self
            .transcode
            .as_ref()
            .and_then(|t| (t.queued_bytes() > 0).then_some(now));
        match self.targets.get(event.rendition) {
            Some(Target::Main) => Some(event.output),
            Some(Target::Output(i)) => {
                if let Some(o) = self.outputs.get_mut(*i) {
                    o.take(&event.output);
                }
                None
            }
            None => None,
        }
    }

    /// Puts into a rendition's video unit the caption tuples of the
    /// stream's frames since its last, up to its time; see
    /// [`ingress`](super#renditions).
    fn carry_captions(&mut self, event: &mut CodecEvent) {
        let Some(video) = self.output_video.get_mut(event.rendition) else {
            return;
        };
        let u = match &mut event.output {
            Output::Config(c) => {
                if c.kind == TrackKind::Video {
                    *video = NalFormat::of(&c.params).map(|f| (c.track, c.timescale, f));
                }
                return;
            }
            Output::Unit(u) => u,
        };
        let Some((track, timescale, nals)) = *video else {
            return;
        };
        if u.track != track {
            return;
        }
        let tuples = self.captions.take(event.rendition, u.pts, timescale);
        if tuples.is_empty() {
            return;
        }
        let mut payload = Vec::with_capacity(u.payload.len() + 16 + 3 * tuples.len());
        match nals.with_cc_sei(&u.payload, &tuples, &mut payload) {
            Ok(()) => u.payload = payload,
            Err(e) => tracing::debug!(error = %e, "a rendition's frame took no captions"),
        }
    }

    /// Ends the transcode that failed, or ended, or stalled at `now`, for
    /// `why`: it starts again at the stream's next keyframe, or, past its
    /// restarts, the renditions are given up.
    pub(super) fn failed(&mut self, why: &str, now: Time, m: &mut TranscodeMetrics) -> Report {
        self.transcode = None;
        self.stalled_since = None;
        self.await_keyframe = true;
        tracing::warn!(reason = why, "a transcode stopped");
        let window = self.settings.restart_window;
        let recent = self.starts.iter().filter(|&&t| now - t <= window).count();
        if recent > self.settings.max_restarts {
            self.abandoned = true;
            self.plan = None;
            m.abandoned += 1;
            return Report::Abandoned(format!("the stream's transcode failed: {why}"));
        }
        Report::None
    }

    /// When the transcode, holding input without output, is due to be
    /// taken as stalled.
    pub(super) fn stall_due(&self) -> Option<Time> {
        self.transcode
            .as_ref()
            .and(self.stalled_since)
            .map(|t| t + self.settings.stall_timeout)
    }

    /// When an output's partial integrity unit is due for signing.
    pub(super) fn sign_due(&self) -> Option<Time> {
        self.outputs
            .iter()
            .filter_map(RenditionOutput::sign_due)
            .min()
    }

    /// Ends the transcode's input.
    pub(super) fn finish_input(&mut self) {
        if let Some(t) = &mut self.transcode {
            t.finish();
        }
    }

    /// Counts the outputs' records into `m`: whether a listing changed.
    pub(super) fn take_records(&mut self, m: &mut TranscodeMetrics) -> bool {
        let mut changed = false;
        for o in &mut self.outputs {
            let (records, bytes, listed) = o.take_records();
            m.records += records;
            m.bytes += bytes;
            changed |= listed;
        }
        changed
    }

    /// What the snippet lists of the outputs.
    pub(super) fn listed(&self) -> impl Iterator<Item = &ListingRendition> {
        self.outputs.iter().filter_map(|o| o.listed.as_ref())
    }

    /// Outputs whose swarm isn't created yet but has its first config.
    pub(super) fn to_create(&self) -> Vec<usize> {
        self.outputs
            .iter()
            .enumerate()
            .filter(|(_, o)| !o.created && !o.writer.configs().is_empty())
            .map(|(i, _)| i)
            .collect()
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Creates the swarm of `o`, a rendition of the session `session` of
    /// channel `channel_id`, as its channel's swarm is
    /// ([`create_ingest_swarm`](Self::create_ingest_swarm)): in the slot
    /// of its transfer, whoever's it is, or a new one.
    pub(super) async fn create_rendition_swarm(
        &self,
        channel_id: u64,
        session: u64,
        o: &mut RenditionOutput<<H as SwarmHost>::Writer>,
    ) {
        o.created = true;
        let network_key = {
            let ing = self.ingress.borrow();
            ing.sessions
                .get(&channel_id)
                .filter(|s| s.id == session)
                .map(|s| s.network_key.clone())
        };
        let Some(network_key) = network_key else {
            return;
        };
        let networks = BTreeSet::from([network_key]);
        let options = o.options();
        let created = self
            .host
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key: o.key.clone(),
            })
            .await;
        let (swarm, id, writer) = match created {
            Ok(c) => c,
            Err(e) => {
                self.host_failed("creating a rendition swarm", &e);
                return;
            }
        };
        if id != o.swarm_id || !self.is_current(channel_id, session) {
            self.remove_host_swarm(swarm).await;
            return;
        }
        let t = o.t;
        let (slot, stale) = self.with_swarms(|r| {
            match r.replace(
                &t,
                id.clone(),
                options.clone(),
                SwarmKind::Ingest,
                networks.clone(),
            ) {
                Some((slot, old)) => {
                    r.set_swarm(&t, slot, swarm);
                    (slot, old)
                }
                None => {
                    let claim = r.insert(
                        t,
                        Some(swarm),
                        id.clone(),
                        options.clone(),
                        SwarmKind::Ingest,
                        networks,
                        0,
                    );
                    (claim.slot, None)
                }
            }
        });
        o.slot = Some(slot);
        if stale.is_some() {
            self.transfers.remove(t);
        }
        self.transfers.add(swarm, &id, Vec::new());
        self.publish_joined(&t);
        if let Some(old) = stale {
            self.remove_host_swarm(old).await;
        }
        {
            let mut ing = self.ingress.borrow_mut();
            if let Some(s) = ing
                .sessions
                .get_mut(&channel_id)
                .filter(|s| s.id == session)
            {
                s.renditions.insert(t, slot);
            }
        }
        let uri = Uri::new(id, options.uri_options()).to_string();
        o.swarm = Some(IngestSwarm { writer, uri });
    }

    /// Takes the rendition swarms `slots` names out of the node: their
    /// transfers, slots and swarms.
    pub(super) async fn remove_renditions(&self, slots: Vec<(TransferId, u64)>) {
        for (t, slot) in slots {
            self.transfers.remove(t);
            let e = self.with_swarms(|r| r.remove_slot(&t, slot));
            if let Some(swarm) = e.and_then(|e| e.swarm) {
                self.remove_host_swarm(swarm).await;
            }
        }
    }
}

/// The video codecs a session's stream may be in, which it keeps.
fn video_codec(params: &CodecParams) -> u8 {
    match params {
        CodecParams::Hevc { .. } => 1,
        CodecParams::Av1 { .. } => 2,
        _ => 0,
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Live<K, H, V, T> {
    /// Plans the session's renditions at its stream's first video config
    /// `c`; see [`ingress`](super#renditions). A video config of another
    /// codec later ends the session.
    pub(super) fn plan_renditions(&mut self, c: &DecoderConfig) -> Result<(), End> {
        if c.kind != TrackKind::Video {
            return Ok(());
        }
        let codec = video_codec(&c.params);
        if let Some(planned) = self.video_codec {
            if planned != codec {
                return Err(End::Media("the stream's video codec changed".into()));
            }
            return Ok(());
        }
        self.video_codec = Some(codec);
        let s = self.publication.shared.clone();
        let settings = &s.config.ingress.transcode;
        let planned = plan(c, settings);
        let stage = s.video.codec();
        if planned.main_transcoded && stage.is_none() {
            return Err(End::Media(untranscoded(c).unwrap_or_default()));
        }
        let (Some(stage), Some(transcode)) = (stage, &planned.transcode) else {
            return Ok(());
        };
        let running = s.ingress.borrow().transcodes.clone();
        let Some(place) = TranscodePlace::take(&running, settings.max_sessions) else {
            s.metrics.borrow_mut().transcode.over_cap += 1;
            if planned.main_transcoded {
                return Err(End::Media(format!(
                    "{} needs transcoding, and this node transcodes no more streams; \
                     send H.264 with AAC",
                    c.codec
                )));
            }
            return Ok(());
        };
        let key = {
            let ing = s.ingress.borrow();
            ing.sessions
                .get(&self.publication.channel_id)
                .filter(|e| e.id == self.publication.session)
                .map(|e| e.key.clone())
        };
        let Some(key) = key else {
            return Err(End::Stopped(REPLACED.into()));
        };
        let nominal = match &self.sign {
            Sign::Choosing(choice) => choice.nominal(),
            Sign::Timer(_) => None,
        };
        self.main_bitrate = if planned.main_transcoded {
            transcode
                .renditions
                .first()
                .map(|r| r.bitrate.saturating_add(AUDIO_BPS))
        } else {
            nominal
        };
        self.main_from_source = !planned.main_transcoded;
        self.renditions = Some(Renditions::new(
            stage,
            settings,
            &planned,
            Some(place),
            &key,
            nominal,
            self.frame_rate,
            self.writer.frame_size(),
        ));
        Ok(())
    }

    /// Takes the transcode's next event, or its end, at `now`.
    pub(super) fn transcoded(
        &mut self,
        e: Option<Result<CodecEvent, String>>,
        now: Time,
    ) -> Result<(), End> {
        let Some(r) = &mut self.renditions else {
            return Ok(());
        };
        let why = match e {
            Some(Ok(event)) => {
                if let Some(output) = r.event(event, now) {
                    let refused = match &output {
                        Output::Config(c) => self.writer.set_config(c),
                        Output::Unit(u) => self.writer.push(&u.as_access_unit()),
                    };
                    if let Err(e) = refused {
                        tracing::debug!(error = %e, "the channel's writer refused a transcoded event");
                    }
                }
                return Ok(());
            }
            Some(Err(e)) => e,
            None => "it ended".to_owned(),
        };
        let s = self.publication.shared.clone();
        let report = {
            let mut m = s.metrics.borrow_mut();
            m.transcode.failures += 1;
            r.failed(&why, now, &mut m.transcode)
        };
        self.reported(report)
    }

    /// Acts on what the renditions report: an abandoned transcode ends a
    /// session whose channel swarm carried it, and otherwise ends the
    /// renditions.
    fn reported(&mut self, report: Report) -> Result<(), End> {
        match report {
            Report::None => Ok(()),
            Report::Abandoned(reason) if !self.main_from_source => Err(End::Media(reason)),
            Report::Abandoned(_) => {
                if let Some(r) = &mut self.renditions {
                    for o in &mut r.outputs {
                        let _ = o.writer.finish();
                        o.listed = None;
                    }
                }
                self.list_renditions();
                Ok(())
            }
        }
    }

    /// When the renditions are next due: an output's signing, or the
    /// transcode's stall.
    pub(super) fn renditions_due(&self) -> Option<Time> {
        let r = self.renditions.as_ref()?;
        [r.sign_due(), r.stall_due()].into_iter().flatten().min()
    }

    /// Does what the renditions have due at `now`: a stalled transcode
    /// stops, to start again.
    pub(super) fn renditions_timer(&mut self, now: Time) -> Result<(), End> {
        let Some(r) = &mut self.renditions else {
            return Ok(());
        };
        if !r.stall_due().is_some_and(|t| now >= t) {
            return Ok(());
        }
        let s = self.publication.shared.clone();
        let report = {
            let mut m = s.metrics.borrow_mut();
            m.transcode.stalls += 1;
            r.failed("it stalled", now, &mut m.transcode)
        };
        self.reported(report)
    }

    /// Creates the rendition swarms that have their first config, writes
    /// the outputs, has their partial units signed when due, and lists
    /// them in the snippet.
    pub(super) async fn settle_renditions(&mut self, now: Time) {
        let s = self.publication.shared.clone();
        let (channel_id, session) = (self.publication.channel_id, self.publication.session);
        let Some(r) = &mut self.renditions else {
            return;
        };
        for i in r.to_create() {
            if let Some(o) = r.outputs.get_mut(i) {
                s.create_rendition_swarm(channel_id, session, o).await;
            }
        }
        for o in &mut r.outputs {
            let mut res = o.write(now).await;
            if res.is_ok() && o.sign_due().is_some_and(|t| now >= t) {
                res = o.sign(now).await;
            }
            if let Err(e) = res {
                s.count(|m| m.write_errors += 1);
                tracing::warn!(rendition = %o.id, error = %e, "writing a rendition failed");
            }
        }
        let changed = r.take_records(&mut s.metrics.borrow_mut().transcode);
        if changed || self.listed_main.is_none() {
            self.list_renditions();
        }
    }

    /// Lists the renditions in the session's snippet once the channel's
    /// swarm has records: the channel's own, then the outputs'.
    fn list_renditions(&mut self) {
        let Some(r) = &self.renditions else {
            return;
        };
        let (Some(swarm), Some(_)) = (&self.swarm, &self.mime) else {
            return;
        };
        let main = listing(
            self.writer.configs(),
            &swarm.uri,
            self.main_bitrate.unwrap_or(0),
            self.frame_rate,
        );
        let others: Vec<ListingRendition> = r.listed().cloned().collect();
        // A stream of one rendition lists none.
        let listed = if others.is_empty() {
            Vec::new()
        } else {
            std::iter::once(main.clone()).chain(others).collect()
        };
        self.listed_main = Some(main);
        let s = self.publication.shared.clone();
        s.set_listed_renditions(
            self.publication.channel_id,
            self.publication.session,
            listed,
        );
    }

    /// Ends the renditions at the session's end: the transcode writes what
    /// it holds, within the finish timeout, and each output its last
    /// partial unit, after its END when `end`.
    pub(super) async fn finish_renditions(&mut self, end: bool) {
        let s = self.publication.shared.clone();
        let Some(r) = &mut self.renditions else {
            return;
        };
        r.finish_input();
        let deadline = finish_deadline(s.timer.now(), &s.config.ingress.transcode);
        while r.transcode.is_some() {
            let next = {
                let event = pin!(poll_fn(|cx| r.poll_event(cx)));
                let sleep = pin!(s.timer.sleep_until(deadline));
                match select(event, sleep).await {
                    Either::Left((Some(Ok(e)), _)) => Some(e),
                    Either::Left(_) | Either::Right(_) => None,
                }
            };
            let Some(event) = next else {
                break;
            };
            if let Some(output) = r.event(event, s.timer.now()) {
                let refused = match &output {
                    Output::Config(c) => self.writer.set_config(c),
                    Output::Unit(u) => self.writer.push(&u.as_access_unit()),
                };
                if let Err(e) = refused {
                    tracing::debug!(error = %e, "the channel's writer refused a transcoded event");
                }
            }
        }
        r.transcode = None;
        let now = s.timer.now();
        for o in &mut r.outputs {
            if end {
                let _ = o.writer.finish();
            }
            let mut res = o.write(now).await;
            if res.is_ok() && o.partial() {
                res = o.sign(now).await;
            }
            if let Err(e) = res {
                tracing::warn!(rendition = %o.id, error = %e, "ending a rendition failed");
            }
        }
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Lists `listed` as the renditions of the snippet of session `session`
    /// of channel `channel_id`, and serves the snippet again if they
    /// changed.
    fn set_listed_renditions(&self, channel_id: u64, session: u64, listed: Vec<ListingRendition>) {
        let pushed = {
            let mut ing = self.ingress.borrow_mut();
            let Some(e) = ing
                .sessions
                .get_mut(&channel_id)
                .filter(|e| e.id == session && e.live)
            else {
                return;
            };
            if e.listed == listed {
                return;
            }
            e.listed = listed;
            (super::session_snippet(e), e.key.clone(), e.swarm_id.clone())
        };
        let (snippet, key, swarm_id) = pushed;
        if let Err(e) = self.push_live_snippet(&swarm_id, &snippet, &key) {
            tracing::warn!(error = %e, channel = channel_id, "serving a snippet's renditions failed");
        }
    }
}

/// A rendition plan for tests: one rung.
#[cfg(test)]
pub(super) fn rung(short_side: u16, max_frame_rate: u32, bitrate: u64) -> RenditionPlan {
    RenditionPlan {
        short_side,
        max_frame_rate,
        bitrate,
    }
}

/// How long a session's end waits for its transcode's output.
pub(super) fn finish_deadline(now: Time, settings: &TranscodeSettings) -> Time {
    now + settings.finish_timeout.max(Duration::ZERO)
}

#[cfg(test)]
#[path = "renditions_test.rs"]
mod tests;
