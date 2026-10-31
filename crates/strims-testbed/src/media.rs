// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The testbed's media stream: an FLV file's H.264 and AAC, written by the
//! seed in the unit format and read by viewers as the UI's egress reads
//! it, so browser viewers can play it through the baseline player.
//!
//! - **The seed** ([`write()`]) replays the file's tags at their tag times
//!   from the stream's start: each goes through strims-video's
//!   [`FlvSource`] into a [`UnitWriter`] in the swarm's chunk size, its
//!   publish time the wall clock as the tag is taken, and the output goes
//!   into the swarm at once. A partial integrity unit is signed its
//!   signing's interval after its first byte ([`SignTimer`]), and the last
//!   one when the stream ends. Its swarm has the options of a unit-format
//!   ingest session ([`seed_options`]).
//! - **A viewer** ([`read()`]) adds the swarm with an egress stream's
//!   options ([`viewer_options`]) and reads it through a
//!   [`ChunkedReader`], which joins at a GOP as egress does: a spare swarm
//!   reader rewinds there within [`EgressSettings::rewind_timeout`] while
//!   the reader reads on to the next sync record, and the join takes the
//!   rewind if it lands within [`EgressSettings::rewind_grace`] of that
//!   record's arrival. The records' pieces go out
//!   as the Egress RPC's Data messages ([`DataMessages`]) to
//!   [`Runtime::media`], and each unit record's lag from its publish time
//!   to its arrival goes to the second's line.
//!
//! Every node takes the stream's chunks per signature
//! ([`Stream::Units`](crate::node::Stream::Units)), as a viewer takes them
//! from a listing's swarm URI; the seed refuses a file whose signing,
//! chosen as ingest chooses it ([`signing_of`]), has another.

use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

use strims_base::time::Time;
use strims_ppspp::SwarmOptions;
use strims_transfer::{ReadFailure, SwarmRead, SwarmWrite};
use strims_video::amf0::{self, Metadata, Value};
use strims_video::units::record::{GOP_HEAD_LEN, Record, parse_record};
use strims_video::units::{SignTimer, UnitSigning, UnitWriterMetrics};
use strims_video::{
    ChunkedReader, EgressPiece, EgressSettings, FlvSource, Muxer, ReaderMetrics, Rewind,
    RtmpSettings, SourceEvent, UnitWriter, VideoError, unit_ingress_options,
};

use crate::node::Runtime;

/// An FLV tag's kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TagKind {
    Audio,
    Video,
    /// A script data tag, such as `onMetaData`.
    Script,
}

/// One tag of an [`FlvFile`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Tag {
    pub kind: TagKind,
    /// The tag time in milliseconds, with its extended byte.
    pub ts_ms: u32,
    /// The body's bytes in the file.
    body: Range<usize>,
}

/// An FLV file's audio, video and script tags, in file order.
///
/// Tags of other types are skipped. A tag's body is at most
/// [`RtmpSettings::max_media_message`], the most an RTMP ingest takes.
#[derive(Clone, Debug)]
pub struct FlvFile {
    bytes: Vec<u8>,
    /// Bound: one per 15 bytes of the file, a tag's header and size.
    tags: Vec<Tag>,
}

/// The FLV file header's length, and a tag header's.
const FILE_HEADER_LEN: usize = 9;
const TAG_HEADER_LEN: usize = 11;
/// The FLV tag types the file's tags take.
const TAG_AUDIO: u8 = 8;
const TAG_VIDEO: u8 = 9;
const TAG_SCRIPT: u8 = 18;

impl FlvFile {
    /// Reads `bytes` as an FLV file: the `FLV` header, then tags, each
    /// followed by its previous-tag-size field. A file cut inside its last
    /// tag keeps the tags before it.
    pub fn parse(bytes: Vec<u8>) -> Result<FlvFile, String> {
        if bytes.get(..3) != Some(b"FLV") {
            return Err("not an FLV file".to_owned());
        }
        let be = |r: Range<usize>| {
            bytes
                .get(r)
                .map(|b| b.iter().fold(0usize, |v, &x| v << 8 | usize::from(x)))
        };
        let header = be(5..9).ok_or("a short FLV header")?;
        if header < FILE_HEADER_LEN {
            return Err(format!("an FLV header of {header} bytes"));
        }
        let max_body = RtmpSettings::default().max_media_message;
        let mut tags = Vec::new();
        // The first previous-tag-size field follows the header.
        let mut at = header.saturating_add(4);
        while let Some(head) = bytes.get(at..at.saturating_add(TAG_HEADER_LEN)) {
            let kind = head.first().copied().unwrap_or(0);
            let size = be(at + 1..at + 4).unwrap_or(0);
            let ts = be(at + 4..at + 7).unwrap_or(0) as u32
                | u32::from(head.get(7).copied().unwrap_or(0)) << 24;
            if size > max_body {
                return Err(format!("a tag of {size} bytes at offset {at}"));
            }
            let start = at + TAG_HEADER_LEN;
            let end = start + size;
            if end > bytes.len() {
                break;
            }
            // The filter bit and the reserved bits above the type are 0 in
            // plain files.
            let kind = match kind & 0x1f {
                TAG_AUDIO => Some(TagKind::Audio),
                TAG_VIDEO => Some(TagKind::Video),
                TAG_SCRIPT => Some(TagKind::Script),
                _ => None,
            };
            if let Some(kind) = kind {
                tags.push(Tag {
                    kind,
                    ts_ms: ts,
                    body: start..end,
                });
            }
            at = end.saturating_add(4);
        }
        Ok(FlvFile { bytes, tags })
    }

    pub fn tags(&self) -> &[Tag] {
        &self.tags
    }

    /// A tag's body.
    pub fn body(&self, tag: &Tag) -> &[u8] {
        self.bytes.get(tag.body.clone()).unwrap_or_default()
    }

    /// The first `onMetaData` the file's script tags carry.
    pub fn metadata(&self) -> Option<Metadata> {
        self.tags
            .iter()
            .filter(|t| t.kind == TagKind::Script)
            .find_map(|t| {
                let values = amf0::decode_all(self.body(t)).ok()?;
                match values.as_slice() {
                    [Value::String(name), v, ..] if *name == "onMetaData" => {
                        Metadata::from_value(v)
                    }
                    _ => None,
                }
            })
    }

    /// Whether the file has tags of `kind`.
    pub fn has(&self, kind: TagKind) -> bool {
        self.tags.iter().any(|t| t.kind == kind)
    }
}

/// The signing a unit-format ingest session gives a stream of `meta`'s
/// `onMetaData` whose tracks are `video` and `audio`: the metadata's video
/// and audio bitrates are its nominal bitrate when they reach
/// [`UnitSigning::SMALL_BELOW_BPS`] or every track has one
/// ([`UnitSigning::for_stream`]).
pub fn signing_of(meta: Option<&Metadata>, video: bool, audio: bool) -> UnitSigning {
    // A float to integer cast saturates.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let bps = |kbps: Option<f64>| {
        kbps.filter(|r| r.is_finite() && *r > 0.0)
            .map(|r| (r * 1000.0) as u64)
            .filter(|&b| b > 0)
    };
    let video_bps = meta.and_then(|m| bps(m.videodatarate));
    let audio_bps = meta.and_then(|m| bps(m.audiodatarate));
    let sum = video_bps
        .unwrap_or(0)
        .saturating_add(audio_bps.unwrap_or(0));
    let whole = (!video || video_bps.is_some()) && (!audio || audio_bps.is_some());
    let nominal = (whole || sum >= UnitSigning::SMALL_BELOW_BPS).then_some(sum);
    UnitSigning::for_stream(nominal, audio && !video)
}

/// The signing of `chunks_per_signature`: [`UnitSigning::SMALL`] or
/// [`UnitSigning::LARGE`], whichever has it.
pub fn signing_for(chunks_per_signature: usize) -> Result<UnitSigning, String> {
    [UnitSigning::SMALL, UnitSigning::LARGE]
        .into_iter()
        .find(|s| s.chunks_per_signature == chunks_per_signature)
        .ok_or_else(|| format!("no unit signing has {chunks_per_signature} chunks per signature"))
}

/// The seed's swarm options: a unit-format ingest session's.
pub fn seed_options(chunks_per_signature: usize) -> Result<SwarmOptions, String> {
    Ok(unit_ingress_options(&signing_for(chunks_per_signature)?))
}

/// A viewer's swarm options: the seed's, with the live window an egress
/// stream adds a swarm with, [`EgressSettings::window_bytes`] in chunks.
pub fn viewer_options(chunks_per_signature: usize) -> Result<SwarmOptions, String> {
    let mut options = seed_options(chunks_per_signature)?;
    let chunks = EgressSettings::default().window_bytes / options.chunk_size.max(1);
    options.live_window = 1usize << chunks.max(1).ilog2();
    Ok(options)
}

/// What the seed has written.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SeedStats {
    pub writer: UnitWriterMetrics,
    /// Partial integrity units signed at their interval or at the end.
    pub early_signs: u64,
    /// Units the source dropped: those before their track's config.
    pub dropped: u64,
}

impl SeedStats {
    pub fn json(&self) -> String {
        let w = &self.writer;
        format!(
            "{{\"records\":{},\"units\":{},\"gops\":{},\"bytes\":{},\"padding\":{},\"early_signs\":{},\"dropped\":{}}}",
            w.records,
            w.units,
            w.gops,
            w.bytes,
            w.padding,
            self.early_signs,
            self.dropped + w.dropped_before_start,
        )
    }
}

/// The seed's unit writer and its swarm.
struct Seed<'a, W> {
    swarm: &'a mut W,
    units: UnitWriter,
    sign: SignTimer,
    early_signs: u64,
}

impl<W: SwarmWrite> Seed<'_, W> {
    /// Writes the writer's output into the swarm at `now`.
    async fn write_output(&mut self, now: Time) -> Result<(), String> {
        let out = self.units.take_output();
        let res = if out.is_empty() {
            Ok(())
        } else {
            self.swarm.write(&out).await
        };
        self.units.restore(out);
        res.map_err(|e| format!("write: {e}"))?;
        self.sign.written(self.units.offset(), now);
        Ok(())
    }

    /// Has the swarm sign the partial integrity unit at `now`, and goes on
    /// where the next unit starts.
    async fn sign(&mut self, now: Time) -> Result<(), String> {
        self.units.end_frame();
        self.write_output(now).await?;
        let next = self
            .swarm
            .flush()
            .await
            .map_err(|e| format!("flush: {e}"))?;
        self.units
            .continue_at(next)
            .map_err(|e| format!("continue: {e}"))?;
        self.sign.flushed(next);
        self.early_signs += 1;
        Ok(())
    }
}

/// Writes `file`'s tags into the swarm from `start_ms` until `end_ms` (see
/// the [module docs](self)), the latest a tag was taken past its time in
/// `late`, and what was written in `stats`.
#[allow(clippy::too_many_arguments)]
pub async fn write<W: SwarmWrite>(
    swarm: &mut W,
    chunk_size: usize,
    signing: UnitSigning,
    file: &FlvFile,
    start_ms: u64,
    end_ms: u64,
    rt: &impl Runtime,
    late: &Cell<Option<u64>>,
    stats: &Cell<SeedStats>,
) -> Result<(), String> {
    let units = UnitWriter::new(chunk_size).map_err(|e| format!("unit writer: {e}"))?;
    let mut seed = Seed {
        swarm,
        units,
        sign: signing.timer(chunk_size),
        early_signs: 0,
    };
    let mut source = FlvSource::new();
    // Tags are in time order, but the first of each track may not be.
    let Some(first) = file.tags().iter().map(|t| t.ts_ms).min() else {
        return Ok(());
    };
    let unix = |ms: u64| Time::from_unix_millis(i64::try_from(ms).unwrap_or(i64::MAX));
    for tag in file.tags() {
        let at = start_ms + u64::from(tag.ts_ms - first);
        if at >= end_ms {
            break;
        }
        while let Some(due) = seed.sign.due() {
            let due_ms = u64::try_from(due.unix_millis()).unwrap_or(0);
            if due_ms > at {
                break;
            }
            rt.sleep_until_ms(due_ms).await;
            seed.sign(unix(rt.now_ms())).await?;
        }
        rt.sleep_until_ms(at).await;
        let t = rt.now_ms();
        late.set(Some(late.get().unwrap_or(0).max(t.saturating_sub(at))));
        let now = unix(t);
        let body = file.body(tag);
        let event = match tag.kind {
            TagKind::Video => source.video(tag.ts_ms, body, now),
            TagKind::Audio => source.audio(tag.ts_ms, body, now),
            TagKind::Script => continue,
        };
        match event.map_err(|e| format!("source: {e}"))? {
            None => {}
            Some(SourceEvent::Config(c)) => seed
                .units
                .set_config(&c)
                .map_err(|e| format!("config: {e}"))?,
            Some(SourceEvent::Unit(u)) => seed.units.push(&u).map_err(|e| format!("unit: {e}"))?,
        }
        seed.write_output(now).await?;
        stats.set(SeedStats {
            writer: seed.units.metrics(),
            early_signs: seed.early_signs,
            dropped: source.metrics().before_config,
        });
    }
    if seed.sign.is_partial() {
        seed.sign(unix(rt.now_ms())).await?;
    }
    stats.set(SeedStats {
        writer: seed.units.metrics(),
        early_signs: seed.early_signs,
        dropped: source.metrics().before_config,
    });
    Ok(())
}

/// How a viewer's join rewind went.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RewindOutcome {
    Granted,
    /// The swarm refused it, or the reader failed.
    Refused,
    /// [`EgressSettings::rewind_timeout`] passed first.
    TimedOut,
    /// The next sync record arrived and the rewind's grace passed first.
    Overtaken,
}

impl RewindOutcome {
    fn name(self) -> &'static str {
        match self {
            RewindOutcome::Granted => "granted",
            RewindOutcome::Refused => "refused",
            RewindOutcome::TimedOut => "timed_out",
            RewindOutcome::Overtaken => "overtaken",
        }
    }
}

/// A viewer's join, on the wall clock in Unix milliseconds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Join {
    /// When the swarm reader returned its first bytes.
    pub first_read_ms: Option<u64>,
    /// The first rewind: when it started and ended, how it went, and how
    /// far back it went from the reader's position, in bytes.
    pub rewind_start_ms: Option<u64>,
    pub rewind_end_ms: Option<u64>,
    pub rewind: Option<RewindOutcome>,
    pub rewind_back: Option<u64>,
}

/// What a viewer has read.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ViewerStats {
    /// Unit records read whole, and the keyframes among them.
    pub units: u64,
    pub keyframes: u64,
    /// Data messages, and their bytes.
    pub messages: u64,
    pub bytes: u64,
    /// When the first Data message went out, in Unix milliseconds.
    pub first_message_ms: Option<u64>,
    pub reader: ReaderMetrics,
    pub join: Join,
}

impl ViewerStats {
    pub fn json(&self) -> String {
        let r = &self.reader;
        let j = &self.join;
        format!(
            "{{\"units\":{},\"keyframes\":{},\"messages\":{},\"bytes\":{},\"first_message_ms\":{},\"discontinuities\":{},\"corrupt_frames\":{},\"losses\":{},\"records\":{},\"rewinds\":{},\"refused_rewinds\":{},\"overtaken_rewinds\":{},\"first_read_ms\":{},\"rewind_start_ms\":{},\"rewind_end_ms\":{},\"rewind\":{},\"rewind_back\":{}}}",
            self.units,
            self.keyframes,
            self.messages,
            self.bytes,
            opt(self.first_message_ms),
            r.discontinuities,
            r.corrupt_frames,
            r.losses,
            r.records,
            r.rewinds,
            r.refused_rewinds,
            r.overtaken_rewinds,
            opt(j.first_read_ms),
            opt(j.rewind_start_ms),
            opt(j.rewind_end_ms),
            j.rewind
                .map_or("null".to_owned(), |o| format!("\"{}\"", o.name())),
            opt(j.rewind_back),
        )
    }
}

fn opt(v: Option<u64>) -> String {
    v.map_or("null".to_owned(), |v| v.to_string())
}

/// The swarm reader with a deadline and a grace on its rewinds, noting the
/// join. A stream's reader and its spare share what they note.
struct Deadline<S, R> {
    reader: S,
    rt: R,
    timeout_ms: u64,
    grace_ms: u64,
    /// The stream offset just past the last read.
    next: Rc<Cell<u64>>,
    join: Rc<Cell<Join>>,
}

/// Notes a viewer's first rewind as it ends, or as overtaken when it is
/// dropped first.
struct Noting<R: Runtime> {
    rt: R,
    join: Rc<Cell<Join>>,
    start: u64,
    back: u64,
    outcome: RewindOutcome,
}

impl<R: Runtime> Drop for Noting<R> {
    fn drop(&mut self) {
        let mut j = self.join.get();
        if j.rewind.is_none() {
            j.rewind_start_ms = Some(self.start);
            j.rewind_end_ms = Some(self.rt.now_ms());
            j.rewind = Some(self.outcome);
            j.rewind_back = Some(self.back);
            self.join.set(j);
        }
    }
}

impl<S: SwarmRead, R: Runtime + 'static> SwarmRead for Deadline<S, R> {
    type Error = S::Error;

    async fn read_at(&mut self, out: &mut [u8]) -> Result<(u64, usize), ReadFailure<S::Error>> {
        let r = SwarmRead::read_at(&mut self.reader, out).await;
        if let Ok((offset, n)) = r {
            self.next.set(offset + n as u64);
            let mut j = self.join.get();
            if j.first_read_ms.is_none() {
                j.first_read_ms = Some(self.rt.now_ms());
                self.join.set(j);
            }
        }
        r
    }
}

impl<S: SwarmRead, R: Runtime + Clone + 'static> Rewind for Deadline<S, R> {
    async fn rewind(&mut self, offset: u64) -> Option<u64> {
        let start = self.rt.now_ms();
        let mut noting = Noting {
            rt: self.rt.clone(),
            join: self.join.clone(),
            start,
            back: self.next.get().saturating_sub(offset),
            outcome: RewindOutcome::Overtaken,
        };
        let deadline = start + self.timeout_ms;
        let (at, outcome) = tokio::select! {
            biased;
            r = SwarmRead::rewind(&mut self.reader, offset) => match r {
                Ok(at) => (Some(at), RewindOutcome::Granted),
                Err(_) => (None, RewindOutcome::Refused),
            },
            () = self.rt.sleep_until_ms(deadline) => (None, RewindOutcome::TimedOut),
        };
        noting.outcome = outcome;
        at
    }

    fn grace(&self) -> impl Future<Output = ()> + 'static {
        let rt = self.rt.clone();
        let at = rt.now_ms() + self.grace_ms;
        async move { rt.sleep_until_ms(at).await }
    }
}

/// The Egress RPC's Data messages of a stream's pieces: each holds at most
/// `max` bytes of records; the one that completes a record has
/// `segment_end`, and the first after a discontinuity has `discontinuity`
/// and begins a whole record. A message waits until it is full or
/// completes a record.
#[derive(Debug)]
pub struct DataMessages {
    /// The next message's bytes. Bound: `max`.
    data: Vec<u8>,
    max: usize,
    discontinuity: bool,
    /// `data` went out in a message, and the next piece starts another.
    sent: bool,
}

/// A Data message.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DataMessage<'a> {
    pub data: &'a [u8],
    pub segment_end: bool,
    pub discontinuity: bool,
}

impl DataMessages {
    /// Messages of at most `max` bytes (at least 1).
    pub fn new(max: usize) -> DataMessages {
        let max = max.max(1);
        DataMessages {
            data: Vec::with_capacity(max),
            max,
            discontinuity: false,
            sent: false,
        }
    }

    /// The most bytes the next data piece may hold.
    pub fn room(&self) -> usize {
        if self.sent {
            return self.max;
        }
        self.max.saturating_sub(self.data.len()).max(1)
    }

    /// Takes the next piece, of at most [`room`](Self::room) bytes, and
    /// answers the message it completes.
    pub fn piece(&mut self, piece: EgressPiece<'_>) -> Option<DataMessage<'_>> {
        if std::mem::take(&mut self.sent) {
            self.data.clear();
        }
        let segment_end = match piece {
            EgressPiece::Data(d) => {
                self.data.extend_from_slice(d);
                if self.data.len() < self.max {
                    return None;
                }
                false
            }
            EgressPiece::RecordEnd => true,
            EgressPiece::Discontinuity => {
                self.data.clear();
                self.discontinuity = true;
                return None;
            }
        };
        self.sent = true;
        Some(DataMessage {
            data: &self.data,
            segment_end,
            discontinuity: std::mem::take(&mut self.discontinuity),
        })
    }
}

/// The bytes of a record's start kept to read its header: its kind,
/// `gop_back`, header length and a UNIT header.
const HEAD_KEEP: usize = 128;
const _: () = assert!(HEAD_KEEP > GOP_HEAD_LEN);

/// Reads the stream through `reader` (see the [module docs](self)) until
/// it closes, `spare` (a second reader opened with it) racing its join's
/// rewind: each Data message goes to [`Runtime::media`], each unit
/// record's lag to `lags`, and what was read to `stats`.
pub async fn read<S: SwarmRead, R: Runtime + Clone + 'static>(
    reader: S,
    spare: Option<S>,
    chunk_size: usize,
    rt: &R,
    lags: &RefCell<Vec<u64>>,
    stats: &RefCell<ViewerStats>,
) -> Result<(), String> {
    let egress = EgressSettings::default();
    let join = Rc::new(Cell::new(Join::default()));
    let next = Rc::new(Cell::new(0));
    let deadline = |reader| Deadline {
        reader,
        rt: rt.clone(),
        timeout_ms: u64::try_from(egress.rewind_timeout.as_millis()).unwrap_or(0),
        grace_ms: u64::try_from(egress.rewind_grace.as_millis()).unwrap_or(0),
        next: next.clone(),
        join: join.clone(),
    };
    let mut records = ChunkedReader::new(deadline(reader), chunk_size, egress.read_buffer_bytes)
        .map_err(|e| format!("chunked reader: {e}"))?;
    if let Some(spare) = spare {
        records.set_spare(deadline(spare));
    }
    let mut messages = DataMessages::new(egress.max_data_bytes);
    // The record in progress's first bytes. Bound: HEAD_KEEP.
    let mut head = Vec::with_capacity(HEAD_KEEP);
    loop {
        let piece = match records.next(messages.room()).await {
            Ok(p) => p,
            Err(VideoError::StreamClosed) => return Ok(()),
            Err(e) => {
                rt.error(format!("read: {e}"));
                continue;
            }
        };
        let now = rt.now_ms();
        match piece {
            EgressPiece::Data(d) => {
                let keep = HEAD_KEEP.saturating_sub(head.len()).min(d.len());
                head.extend_from_slice(d.get(..keep).unwrap_or_default());
            }
            EgressPiece::RecordEnd => {
                if let Ok(Record::Unit { header, .. }) = parse_record(&head) {
                    let mut s = stats.borrow_mut();
                    s.units += 1;
                    s.keyframes += u64::from(header.keyframe);
                    let published = u64::try_from(header.published.unix_millis()).unwrap_or(0);
                    lags.borrow_mut().push(now.saturating_sub(published));
                }
                head.clear();
            }
            EgressPiece::Discontinuity => head.clear(),
        }
        if let Some(m) = messages.piece(piece) {
            rt.media(m.data, m.segment_end, m.discontinuity);
            let mut s = stats.borrow_mut();
            s.messages += 1;
            s.bytes += m.data.len() as u64;
            s.first_message_ms.get_or_insert(now);
        }
        let mut s = stats.borrow_mut();
        s.reader = records.metrics();
        s.join = join.get();
    }
}

#[cfg(test)]
#[path = "media_test.rs"]
mod tests;
