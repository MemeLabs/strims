// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A session's live thumbnails: JPEGs of its stream's keyframes that its
//! snippet shows in place of its channel's
//! ([`ingress`](super#thumbnails)).

use std::cell::Cell;
use std::pin::pin;
use std::rc::Rc;

use futures_util::FutureExt;
use futures_util::future::{Either, select};
use strims_base::time::Time;
use strims_directory::Host;
use strims_directory::snippet::MAX_IMAGE_BYTES;
use strims_kv::Kv;
use strims_proto::network::v1::directory::ListingSnippetImage;
use strims_proto::network::v1::directory::listing_snippet_image::SourceOneof;
use strims_proto::types::{Image, ImageType};
use strims_rpc::Timer;

use super::{Live, Shared};
use crate::codec::Codec;
use crate::config::{MIN_THUMBNAIL_WIDTH, ThumbnailSettings};
use crate::host::VideoHost;
use crate::media::{AccessUnit, DecoderConfig, TrackKind};

/// The bytes of an encoded `ListingSnippetImage` besides its JPEG, at
/// most: the oneof's tag and length, and the image's type, size and data
/// tags and lengths.
const IMAGE_OVERHEAD: usize = 32;
/// The most JPEG segments read for the picture's size.
const MAX_JPEG_SEGMENTS: usize = 64;

/// Where a session's thumbnail stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Attempt {
    Idle,
    /// The codec stage is making one.
    Running,
    /// The last one was made, or failed.
    Made,
    Failed,
}

/// When a session makes its next thumbnail, and of what.
pub(super) struct Thumbnails {
    settings: ThumbnailSettings,
    /// The stream's video config, of its own encoding.
    config: Option<DecoderConfig>,
    /// When the next is due, from the stream's first video unit on.
    due: Option<Time>,
    /// When the last attempt began.
    began: Time,
    /// Failures since the last thumbnail made: each halves the width.
    retries: u32,
    /// Shared with the task making one.
    attempt: Rc<Cell<Attempt>>,
}

impl Thumbnails {
    pub(super) fn new(settings: &ThumbnailSettings) -> Thumbnails {
        Thumbnails {
            settings: settings.clone(),
            config: None,
            due: None,
            began: Time::default(),
            retries: 0,
            attempt: Rc::new(Cell::new(Attempt::Idle)),
        }
    }

    /// Takes a config of the stream.
    pub(super) fn config(&mut self, c: &DecoderConfig) {
        if c.kind == TrackKind::Video {
            self.config = Some(c.clone());
        }
    }

    /// Whether a thumbnail is due of `u`, a unit of the stream that came
    /// at `now`: the config it decodes with and the thumbnail's width.
    fn take(&mut self, u: &AccessUnit<'_>, now: Time) -> Option<(DecoderConfig, u16)> {
        let config = self.config.as_ref().filter(|c| c.track == u.track)?;
        match self.attempt.get() {
            Attempt::Running => return None,
            Attempt::Made => {
                self.retries = 0;
                self.due = Some(self.began + self.settings.interval);
            }
            Attempt::Failed => {
                self.retries = self.retries.saturating_add(1);
                self.due = Some(self.began + self.settings.retry);
            }
            Attempt::Idle => {}
        }
        self.attempt.set(Attempt::Idle);
        let due = *self.due.get_or_insert(now + self.settings.first_after);
        if !u.keyframe || now < due {
            return None;
        }
        let width = self.width(config);
        let config = config.clone();
        self.began = now;
        self.attempt.set(Attempt::Running);
        Some((config, width))
    }

    /// The width of a thumbnail of `config`'s picture: the widest within
    /// the settings' box at the picture's aspect ratio, and no wider than
    /// the picture, halved once a retry down to [`MIN_THUMBNAIL_WIDTH`],
    /// and even.
    fn width(&self, config: &DecoderConfig) -> u16 {
        let s = &self.settings;
        let (w, h) = config
            .params
            .picture_size()
            .unwrap_or((s.max_width, s.max_height));
        let (w, h) = (u32::from(w.max(2)), u32::from(h.max(2)));
        let fit = u32::from(s.max_width)
            .min(u32::from(s.max_height) * w / h)
            .min(w);
        let floor = u32::from(MIN_THUMBNAIL_WIDTH).min(fit);
        let width = (fit >> self.retries.min(16)).max(floor) & !1;
        u16::try_from(width.max(2)).unwrap_or(u16::MAX)
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Live<K, H, V, T> {
    /// Makes a thumbnail of `u`, a unit of the stream that came at `now`,
    /// if one is due: in a task, whose thumbnail the session's snippet
    /// then shows.
    pub(super) fn thumbnail(&mut self, u: &AccessUnit<'_>, now: Time) {
        let Some(t) = &mut self.thumbnails else {
            return;
        };
        let s = self.publication.shared.clone();
        let Some(codec) = s.video.codec() else {
            return;
        };
        let Some((config, width)) = t.take(u, now) else {
            return;
        };
        let max_bytes = MAX_IMAGE_BYTES - IMAGE_OVERHEAD;
        let made = codec.thumbnail(std::slice::from_ref(&config), u, width, max_bytes);
        let attempt = t.attempt.clone();
        let deadline = now + t.settings.timeout;
        let (channel_id, session) = (self.publication.channel_id, self.publication.session);
        let task = s.clone();
        task.spawn(
            async move {
                let made = {
                    let sleep = pin!(s.timer.sleep_until(deadline));
                    match select(made, sleep).await {
                        Either::Left((Ok(jpeg), _)) => Ok(jpeg),
                        Either::Left((Err(e), _)) => Err(e.to_string()),
                        Either::Right(_) => Err("it took too long".to_owned()),
                    }
                };
                match made {
                    Ok(jpeg) => {
                        attempt.set(Attempt::Made);
                        s.count(|m| m.thumbnails += 1);
                        s.set_live_thumbnail(channel_id, session, snippet_image(jpeg));
                    }
                    Err(e) => {
                        attempt.set(Attempt::Failed);
                        s.count(|m| m.thumbnail_failures += 1);
                        tracing::debug!(channel = channel_id, error = %e, width, "a thumbnail wasn't made");
                    }
                }
            }
            .boxed_local(),
        );
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Shows `image` as the thumbnail of session `session` of channel
    /// `channel_id`, serving its snippet again, while the session is live.
    fn set_live_thumbnail(&self, channel_id: u64, session: u64, image: ListingSnippetImage) {
        let pushed = {
            let mut ing = self.ingress.borrow_mut();
            let Some(e) = ing
                .sessions
                .get_mut(&channel_id)
                .filter(|e| e.id == session && e.live)
            else {
                return;
            };
            e.thumbnail = Some(image);
            (super::session_snippet(e), e.key.clone(), e.swarm_id.clone())
        };
        let (snippet, key, swarm_id) = pushed;
        if let Err(e) = self.push_live_snippet(&swarm_id, &snippet, &key) {
            tracing::warn!(error = %e, channel = channel_id, "serving a snippet's thumbnail failed");
        }
    }
}

/// The snippet image of `jpeg`, with its picture's size when its frame
/// header gives one.
fn snippet_image(jpeg: Vec<u8>) -> ListingSnippetImage {
    let (width, height) = jpeg_size(&jpeg).unwrap_or((0, 0));
    ListingSnippetImage {
        source_oneof: Some(SourceOneof::Image(Image {
            r#type: ImageType::Jpeg as i32,
            width: u32::from(width),
            height: u32::from(height),
            data: jpeg,
        })),
    }
}

/// The width and height a JPEG's start-of-frame segment states, among its
/// first [`MAX_JPEG_SEGMENTS`] segments.
pub(super) fn jpeg_size(jpeg: &[u8]) -> Option<(u16, u16)> {
    let be16 = |at: usize| -> Option<u16> {
        let b = jpeg.get(at..at.checked_add(2)?)?;
        Some(u16::from_be_bytes([*b.first()?, *b.get(1)?]))
    };
    if jpeg.get(..2) != Some(&[0xff, 0xd8]) {
        return None;
    }
    let mut at = 2;
    for _ in 0..MAX_JPEG_SEGMENTS {
        // Markers may be padded with fill bytes.
        while jpeg.get(at) == Some(&0xff) && jpeg.get(at + 1) == Some(&0xff) {
            at += 1;
        }
        if jpeg.get(at) != Some(&0xff) {
            return None;
        }
        let marker = *jpeg.get(at + 1)?;
        at += 2;
        match marker {
            // Markers without a segment.
            0x01 | 0xd0..=0xd8 => continue,
            // Start of scan: the frame header comes before it.
            0xda | 0xd9 => return None,
            // Every start of frame but DHT, JPG and DAC.
            0xc0..=0xcf if ![0xc4, 0xc8, 0xcc].contains(&marker) => {
                let height = be16(at + 3)?;
                let width = be16(at + 5)?;
                return Some((width, height));
            }
            _ => at = at.checked_add(usize::from(be16(at)?))?,
        }
    }
    None
}

#[cfg(test)]
#[path = "thumbnails_test.rs"]
mod tests;
