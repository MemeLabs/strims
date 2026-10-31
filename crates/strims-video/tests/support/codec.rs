// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A codec stage for the tests: a transcode's renditions are the stream's
//! units as they came, each rendition keeping every keyframe and the
//! frames at least a frame of its rate apart, with a config of its size.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use strims_video::codec::{
    Codec, CodecEvent, Input, Output, Refused, Thumbnail, Transcode, TranscodePlan, Unit,
};
use strims_video::synth::script::avcc;
use strims_video::{AccessUnit, CodecParams, DecoderConfig, TrackKind};

/// The fake's `n`th thumbnail, from 0: a JPEG's first and last bytes
/// around its number.
pub fn fake_jpeg(n: usize) -> Vec<u8> {
    let mut jpeg = b"\xff\xd8fake".to_vec();
    jpeg.extend_from_slice(n.to_string().as_bytes());
    jpeg.extend_from_slice(b"\xff\xd9");
    jpeg
}

#[derive(Default)]
pub struct FakeCodecState {
    /// The plans of the transcodes started, in order.
    pub plans: Vec<TranscodePlan>,
    /// Transcodes take input and make nothing.
    pub stall: bool,
    /// The running transcode ends at its next input.
    pub end_next: bool,
    /// Transcodes that finished, their input ended.
    pub finished: usize,
    /// Thumbnails asked for, and the widths asked.
    pub thumbnails: usize,
    pub thumbnail_widths: Vec<u16>,
    /// Thumbnails to fail before the next is made.
    pub fail_thumbnails: usize,
    waker: Option<Waker>,
}

#[derive(Clone, Default)]
pub struct FakeCodec(pub Rc<RefCell<FakeCodecState>>);

impl FakeCodec {
    pub fn plans(&self) -> Vec<TranscodePlan> {
        self.0.borrow().plans.clone()
    }

    pub fn set_stall(&self, stall: bool) {
        self.0.borrow_mut().stall = stall;
    }

    /// Ends the running transcode at its next input.
    pub fn end_next(&self) {
        let mut s = self.0.borrow_mut();
        s.end_next = true;
        if let Some(w) = s.waker.take() {
            w.wake();
        }
    }
}

pub struct FakeTranscode {
    state: Rc<RefCell<FakeCodecState>>,
    plan: TranscodePlan,
    events: VecDeque<CodecEvent>,
    /// Each rendition's last kept frame's PTS.
    kept: Vec<Option<i64>>,
    video: Option<DecoderConfig>,
    held: usize,
    finished: bool,
    ended: bool,
}

impl Codec for FakeCodec {
    type Error = String;
    type Transcode = FakeTranscode;

    fn transcode(&self, plan: &TranscodePlan) -> Result<FakeTranscode, String> {
        self.0.borrow_mut().plans.push(plan.clone());
        Ok(FakeTranscode {
            state: self.0.clone(),
            plan: plan.clone(),
            events: VecDeque::new(),
            kept: vec![None; plan.renditions.len()],
            video: None,
            held: 0,
            finished: false,
            ended: false,
        })
    }

    fn thumbnail(
        &self,
        configs: &[DecoderConfig],
        keyframe: &AccessUnit<'_>,
        width: u16,
        _: usize,
    ) -> Thumbnail<String> {
        let mut s = self.0.borrow_mut();
        let n = s.thumbnails;
        s.thumbnails += 1;
        s.thumbnail_widths.push(width);
        let fail = s.fail_thumbnails > 0;
        s.fail_thumbnails = s.fail_thumbnails.saturating_sub(1);
        let ok = !fail && keyframe.keyframe && configs.iter().any(|c| c.track == keyframe.track);
        Box::pin(std::future::ready(if ok {
            Ok(fake_jpeg(n))
        } else {
            Err("no picture".to_owned())
        }))
    }
}

impl FakeTranscode {
    fn emit(&mut self, rendition: usize, output: Output) {
        self.events.push_back(CodecEvent { rendition, output });
    }

    /// Rendition `i`'s H.264 config of the stream's video config.
    fn config(&self, i: usize, video: &DecoderConfig) -> Option<DecoderConfig> {
        let (w, h) = video.params.picture_size()?;
        let side = u32::from(self.plan.renditions.get(i)?.short_side);
        let short = u32::from(w.min(h)).max(1);
        let scale = |v: u16| u16::try_from(u32::from(v) * side / short).unwrap_or(u16::MAX) & !1;
        let (w, h) = (scale(w), scale(h));
        Some(DecoderConfig {
            codec: "avc1.64001f".into(),
            params: CodecParams::Avc {
                avcc: avcc(w, h),
                width: w,
                height: h,
            },
            ..video.clone()
        })
    }
}

impl Transcode for FakeTranscode {
    type Error = String;

    fn send(&mut self, input: Input<'_>) -> Result<(), Refused> {
        if self.finished {
            return Ok(());
        }
        if self.state.borrow().stall {
            self.held += 1;
            return Ok(());
        }
        let n = self.plan.renditions.len();
        match input {
            Input::Config(c) if c.kind == TrackKind::Video => {
                self.video = Some(c.clone());
                for i in 0..n {
                    if let Some(config) = self.config(i, c) {
                        self.emit(i, Output::Config(config));
                    }
                }
            }
            Input::Config(c) => {
                for i in 0..n {
                    self.emit(i, Output::Config(c.clone()));
                }
            }
            Input::Unit(u) => {
                let video = self.video.as_ref().is_some_and(|v| v.track == u.track);
                for i in 0..n {
                    if video {
                        let fps = i64::from(self.plan.renditions[i].max_frame_rate.max(1));
                        let interval = 90_000 / fps - 180;
                        let keep = u.keyframe || self.kept[i].is_none_or(|k| u.pts - k >= interval);
                        if !keep {
                            continue;
                        }
                        self.kept[i] = Some(u.pts);
                    }
                    self.emit(i, Output::Unit(Unit::of(u)));
                }
            }
        }
        Ok(())
    }

    fn queued_bytes(&self) -> usize {
        self.held
    }

    fn finish(&mut self) {
        self.finished = true;
        self.state.borrow_mut().finished += 1;
    }

    fn poll_next(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<CodecEvent, String>>> {
        if self.ended {
            return Poll::Ready(None);
        }
        if std::mem::take(&mut self.state.borrow_mut().end_next) {
            self.ended = true;
            return Poll::Ready(Some(Err("the encoder exited".into())));
        }
        if let Some(e) = self.events.pop_front() {
            return Poll::Ready(Some(Ok(e)));
        }
        if self.finished {
            self.ended = true;
            return Poll::Ready(None);
        }
        self.state.borrow_mut().waker = Some(cx.waker().clone());
        Poll::Pending
    }
}
