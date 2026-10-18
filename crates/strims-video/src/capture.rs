// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Browser captures: the UI's recording, written into a video swarm and
//! listed on its networks.
//!
//! [`VideoControl::open_capture`] creates a swarm under a fresh ed25519
//! key ([`capture_options`]: 1 KiB chunks, 32 per signature, one stream, a
//! 32 Ki-chunk window, Merkle/BLAKE2b-256/Ed25519), adds its transfer under
//! the empty salt and publishes it on the capture's networks the node has
//! joined, and on the rest as it joins them. It serves the capture's
//! snippet, signed with the capture key, and keeps a media listing of the
//! capture's MIME type and swarm URI on each network's directory. The
//! capture's id is its transfer id.
//!
//! A capture's MIME type is 1 to
//! [`CaptureSettings::max_mime_bytes`](crate::CaptureSettings::max_mime_bytes)
//! bytes of printable ASCII. Its 1 to
//! [`CaptureSettings::max_network_keys`](crate::CaptureSettings::max_network_keys)
//! network keys each name a network in the profile's store; a network the
//! node hasn't joined lists and carries the capture once it does. Its
//! snippet passes [`strims_directory::snippet::check`]. The node runs at
//! most [`CaptureSettings::max_captures`](crate::CaptureSettings::max_captures)
//! captures.
//!
//! # Records
//!
//! [`VideoControl::append_capture`] writes bytes of the record in
//! progress, and ends the record when asked: records are opaque to the
//! node (the UI's WebM). They go into the swarm in the packed chunkstream
//! layout ([`strims_chunkstream::packed`]) as frames fill or records end;
//! the swarm writer is never flushed, so no padding lands in a record. A
//! frame holds one record end, so a record that would end in the frame of
//! the record before it is dropped: the append fails with
//! [`VideoError::RecordDropped`], and the capture goes on with the next
//! record. Each capture's appends are written one at a time, in the order
//! they were made, by a task of its own: an append whose caller gives up
//! before its turn is dropped, and one under way finishes.
//!
//! [`VideoControl::close_capture`] stops the listing keepers (each
//! unpublishes its listing), stops serving the snippet, and removes the
//! transfer, then the swarm.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::poll_fn;
use std::rc::{Rc, Weak};
use std::task::{Poll, Waker};

use futures_util::FutureExt;
use strims_chunkstream::packed::{DEFAULT_SIZE, PackedEncoder};
use strims_crypto::{SignableMessage, generate_key};
use strims_directory::{Host, ListingKeeper, snippet};
use strims_kv::Kv;
use strims_ppspp::integrity::{
    LiveSignatureAlgorithm, MerkleHashTreeFunction, ProtectionMethod, VerifierOptions,
};
use strims_ppspp::{SwarmId, SwarmOptions, Uri, WriterOptions};
use strims_proto::network::v1::directory::listing::{Content, Media};
use strims_proto::network::v1::directory::{Listing, ListingSnippet};
use strims_proto::types::Key;
use strims_rpc::Timer;
use strims_transfer::{SwarmHost, SwarmWrite, TransferId};

use crate::channels::check_network;
use crate::control::{Shared, Slot, Swarm};
use crate::registry::SwarmKind;
use crate::{VideoControl, VideoError, VideoHost};

/// The options of a capture's swarm; its URI carries every one but the
/// window.
pub fn capture_options() -> SwarmOptions {
    SwarmOptions {
        chunk_size: 1024,
        chunks_per_signature: 32,
        stream_count: 1,
        live_window: 32 * 1024,
        integrity: VerifierOptions {
            protection_method: ProtectionMethod::MERKLE_TREE,
            merkle_hash_tree_function: MerkleHashTreeFunction::BLAKE2B256,
            live_signature_algorithm: LiveSignatureAlgorithm::ED25519,
        },
        ..SwarmOptions::default()
    }
    .assign_defaults(&SwarmOptions::default_options())
}

/// A capture the node runs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureInfo {
    /// Its transfer id.
    pub id: TransferId,
    pub swarm_uri: String,
    pub mime_type: String,
    /// Each of its networks' keys, and the id of its listing on that
    /// network's directory once published.
    pub listings: Vec<(Vec<u8>, Option<u64>)>,
}

/// A capture: its swarm, key and listings, and its appends.
pub(crate) struct Capture<H: Host> {
    swarm: Swarm<H>,
    swarm_id: SwarmId,
    key: Key,
    swarm_uri: String,
    mime_type: String,
    /// A keeper per network key. Bound: the capture's network keys.
    keepers: Vec<(Vec<u8>, ListingKeeper)>,
    appends: Rc<Appends>,
}

impl<H: Host> Capture<H> {
    pub(crate) fn keepers(&self) -> impl Iterator<Item = &ListingKeeper> {
        self.keepers.iter().map(|(_, k)| k)
    }
}

/// An append waiting for its capture's writer.
struct Append {
    id: u64,
    data: Vec<u8>,
    end: bool,
    reply: Rc<Slot<Result<(), VideoError>>>,
}

#[derive(Default)]
struct AppendState {
    /// In order. Bound: the appends whose callers wait for them.
    queue: VecDeque<Append>,
    next_id: u64,
    /// Wakes the writer.
    waker: Option<Waker>,
    closed: bool,
}

/// A capture's appends, between their callers and its writer.
#[derive(Default)]
struct Appends {
    state: RefCell<AppendState>,
}

impl Appends {
    /// Queues an append: its id, or `None` once the capture closed.
    fn push(
        &self,
        data: Vec<u8>,
        end: bool,
        reply: Rc<Slot<Result<(), VideoError>>>,
    ) -> Option<u64> {
        let (id, waker) = {
            let mut s = self.state.borrow_mut();
            if s.closed {
                return None;
            }
            let id = s.next_id;
            s.next_id += 1;
            s.queue.push_back(Append {
                id,
                data,
                end,
                reply,
            });
            (id, s.waker.take())
        };
        if let Some(w) = waker {
            w.wake();
        }
        Some(id)
    }

    /// Drops append `id` if it still waits.
    fn cancel(&self, id: u64) {
        let dropped = {
            let mut s = self.state.borrow_mut();
            let at = s.queue.iter().position(|a| a.id == id);
            at.and_then(|at| s.queue.remove(at))
        };
        drop(dropped);
    }

    /// The next append for the writer; `None` once the capture closed.
    async fn next(&self) -> Option<Append> {
        poll_fn(|cx| {
            let mut s = self.state.borrow_mut();
            if let Some(a) = s.queue.pop_front() {
                return Poll::Ready(Some(a));
            }
            if s.closed {
                return Poll::Ready(None);
            }
            s.waker = Some(cx.waker().clone());
            Poll::Pending
        })
        .await
    }

    /// Ends the appends: those waiting fail, and the writer stops.
    fn close(&self) {
        let (queued, waker) = {
            let mut s = self.state.borrow_mut();
            s.closed = true;
            (std::mem::take(&mut s.queue), s.waker.take())
        };
        for a in queued {
            a.reply.set(Err(VideoError::CaptureNotFound));
        }
        if let Some(w) = waker {
            w.wake();
        }
    }
}

/// Drops its append from the queue if the caller stops waiting first.
struct Queued<'a> {
    appends: &'a Appends,
    id: u64,
}

impl Drop for Queued<'_> {
    fn drop(&mut self) {
        self.appends.cancel(self.id);
    }
}

/// A capture being created, counted against the bound until dropped.
struct Opening<K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: Rc<Shared<K, H, V, T>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Opening<K, H, V, T> {
    fn new(shared: &Rc<Shared<K, H, V, T>>) -> Result<Self, VideoError> {
        let open = shared.captures.borrow().len();
        let max = shared.config.capture.max_captures;
        shared.with_swarms(|r| {
            if open + r.opening_captures >= max {
                return Err(VideoError::TooManyCaptures);
            }
            r.opening_captures += 1;
            Ok(())
        })?;
        Ok(Opening {
            shared: shared.clone(),
        })
    }
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for Opening<K, H, V, T> {
    fn drop(&mut self) {
        self.shared.with_swarms(|r| {
            r.opening_captures = r.opening_captures.saturating_sub(1);
        });
    }
}

/// Closes the capture an open made after its caller stopped waiting.
struct OpenReply<K: Kv, H: Host, V: VideoHost, T: Timer> {
    shared: Rc<Shared<K, H, V, T>>,
    reply: Rc<Slot<Result<TransferId, VideoError>>>,
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Drop for OpenReply<K, H, V, T> {
    fn drop(&mut self) {
        if let Some(Ok(id)) = self.reply.take_now() {
            self.shared.queue_close_capture(&id);
        }
    }
}

/// Checks a capture's MIME type; see the [module docs](self).
fn check_mime(mime: &str, max: usize) -> Result<(), VideoError> {
    let printable = mime.bytes().all(|b| (0x20..0x7f).contains(&b));
    if mime.is_empty() || mime.len() > max || !printable {
        return Err(VideoError::MimeType);
    }
    Ok(())
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> Shared<K, H, V, T> {
    /// Creates a capture for a caller waiting on `reply`, or closes it if
    /// the caller is gone.
    async fn create_capture(
        self: Rc<Self>,
        opening: Opening<K, H, V, T>,
        key: Key,
        mime_type: String,
        network_keys: Vec<Vec<u8>>,
        snippet: ListingSnippet,
        reply: Weak<Slot<Result<TransferId, VideoError>>>,
    ) {
        let res = self
            .start_capture(key, mime_type, network_keys, snippet)
            .await;
        drop(opening);
        match reply.upgrade() {
            Some(reply) => reply.set(res),
            None => {
                if let Ok(id) = res
                    && let Some(swarm) = self.end_capture(&id)
                {
                    self.remove_host_swarm(swarm).await;
                }
            }
        }
    }

    async fn start_capture(
        self: &Rc<Self>,
        key: Key,
        mime_type: String,
        network_keys: Vec<Vec<u8>>,
        mut snippet: ListingSnippet,
    ) -> Result<TransferId, VideoError> {
        let options = capture_options();
        let (swarm, swarm_id, writer) = self
            .host
            .create_swarm(WriterOptions {
                swarm_options: options.clone(),
                key: key.clone(),
            })
            .await
            .map_err(|e| self.host_failed("creating a capture swarm", &e))?;
        let t = TransferId::new(&swarm_id, b"");
        let networks = network_keys.iter().cloned().collect();
        let fresh = self.with_swarms(|r| {
            if r.get(&t).is_some() {
                return false;
            }
            r.insert(
                t,
                Some(swarm),
                swarm_id.clone(),
                options.clone(),
                SwarmKind::Capture,
                networks,
                0,
            );
            true
        });
        if !fresh {
            self.remove_host_swarm(swarm).await;
            return Err(VideoError::Host("the capture's swarm id is in use".into()));
        }
        self.transfers.add(swarm, &swarm_id, Vec::new());
        self.publish_joined(&t);
        let pushed = match snippet.sign_message(&key) {
            Ok(()) => self
                .directory
                .push_snippet(&swarm_id, snippet)
                .map_err(VideoError::from),
            Err(e) => Err(e.into()),
        };
        if let Err(e) = pushed {
            self.transfers.remove(t);
            self.with_swarms(|r| r.remove(&t));
            self.remove_host_swarm(swarm).await;
            return Err(e);
        }

        let swarm_uri = Uri::new(swarm_id.clone(), options.uri_options()).to_string();
        let listing = Listing {
            content: Some(Content::Media(Media {
                mime_type: mime_type.clone(),
                swarm_uri: swarm_uri.clone(),
            })),
        };
        let config = self.config.listing.keeper_config();
        let keepers = network_keys
            .into_iter()
            .map(|k| {
                let keeper = ListingKeeper::new(k.clone(), listing.clone());
                let (run, directory) = (keeper.clone(), self.directory.clone());
                self.spawn(async move { run.run(&directory, config, |_| {}).await }.boxed_local());
                (k, keeper)
            })
            .collect();
        let appends = Rc::new(Appends::default());
        self.spawn(
            self.clone()
                .write_capture(appends.clone(), writer)
                .boxed_local(),
        );
        let prev = self.captures.borrow_mut().insert(
            t,
            Capture {
                swarm,
                swarm_id,
                key,
                swarm_uri,
                mime_type,
                keepers,
                appends,
            },
        );
        drop(prev);
        Ok(t)
    }

    /// Writes a capture's appends into its swarm until it closes.
    async fn write_capture(
        self: Rc<Self>,
        appends: Rc<Appends>,
        mut writer: <H as SwarmHost>::Writer,
    ) {
        // invariant: DEFAULT_SIZE is a valid frame size.
        #[allow(clippy::expect_used)]
        let mut encoder = PackedEncoder::new(DEFAULT_SIZE).expect("the default frame size");
        // Bound: two frames.
        let mut out = Vec::with_capacity(2 * DEFAULT_SIZE);
        let mut failed: Option<String> = None;
        while let Some(a) = appends.next().await {
            let res = match &failed {
                Some(e) => Err(VideoError::StreamWrite(e.clone())),
                None => {
                    let res =
                        write_append(&mut encoder, &mut writer, &mut out, &a.data, a.end).await;
                    match res {
                        Ok(true) => Ok(()),
                        Ok(false) => {
                            self.metrics.borrow_mut().dropped_records += 1;
                            Err(VideoError::RecordDropped)
                        }
                        Err(e) => {
                            self.metrics.borrow_mut().write_errors += 1;
                            tracing::warn!(error = %e, "writing a capture failed");
                            failed = Some(e.to_string());
                            Err(VideoError::StreamWrite(e.to_string()))
                        }
                    }
                }
            };
            a.reply.set(res);
        }
    }

    /// Takes a capture out of the node, but for its swarm, which it
    /// returns for the caller to remove; see the [module docs](self).
    pub(crate) fn end_capture(&self, id: &TransferId) -> Option<Swarm<H>> {
        let c = self.captures.borrow_mut().remove(id)?;
        for k in c.keepers() {
            k.stop();
        }
        self.directory.delete_snippet(&c.swarm_id);
        self.transfers.remove(*id);
        c.appends.close();
        self.with_swarms(|r| r.remove(id));
        Some(c.swarm)
    }

    /// Closes capture `id`, removing its swarm in a task.
    pub(crate) fn queue_close_capture(self: &Rc<Self>, id: &TransferId) {
        if let Some(swarm) = self.end_capture(id) {
            let s = self.clone();
            self.spawn(async move { s.remove_host_swarm(swarm).await }.boxed_local());
        }
    }
}

/// Writes `data` of the record in progress through `encoder` into
/// `writer`, and ends the record if `end`. Returns false if the end was
/// refused and the record dropped.
async fn write_append<W: SwarmWrite>(
    encoder: &mut PackedEncoder,
    writer: &mut W,
    out: &mut Vec<u8>,
    data: &[u8],
    end: bool,
) -> Result<bool, W::Error> {
    for piece in data.chunks(DEFAULT_SIZE) {
        encoder.write(piece, out);
        if out.len() >= DEFAULT_SIZE {
            let res = writer.write(out).await;
            out.clear();
            res?;
        }
    }
    let mut ended = true;
    if end && encoder.end_record(out).is_err() {
        // A refused end leaves a record begun in this frame, none of whose
        // bytes are out, so it drops whole.
        encoder.abandon_record();
        ended = false;
    }
    if !out.is_empty() {
        let res = writer.write(out).await;
        out.clear();
        res?;
    }
    Ok(ended)
}

impl<K: Kv, H: Host, V: VideoHost, T: Timer> VideoControl<K, H, V, T> {
    /// Opens a capture of `mime_type`, listed with `snippet` on the
    /// networks `network_keys` names: its id. See the
    /// [module docs](self).
    ///
    /// # Errors
    ///
    /// The MIME type, network keys or snippet are refused, the node runs
    /// its most captures, or a node call failed.
    pub async fn open_capture(
        &self,
        mime_type: &str,
        network_keys: Vec<Vec<u8>>,
        snippet: ListingSnippet,
    ) -> Result<TransferId, VideoError> {
        let s = &self.shared;
        let c = &s.config.capture;
        check_mime(mime_type, c.max_mime_bytes)?;
        if network_keys.is_empty() || network_keys.len() > c.max_network_keys {
            return Err(VideoError::NetworkKeyCount);
        }
        snippet::check(&snippet)?;
        let mut keys: Vec<Vec<u8>> = Vec::new();
        for k in network_keys {
            if !keys.contains(&k) {
                keys.push(k);
            }
        }
        {
            let mut tx = s.store.read().await?;
            for k in &keys {
                check_network(&mut tx, k).await?;
            }
        }
        let opening = Opening::new(s)?;
        let key = generate_key(&mut s.rng.borrow_mut());
        let reply = OpenReply {
            shared: s.clone(),
            reply: Slot::new(),
        };
        let task = s.clone().create_capture(
            opening,
            key,
            mime_type.to_owned(),
            keys,
            snippet,
            Rc::downgrade(&reply.reply),
        );
        s.spawn(task.boxed_local());
        reply.reply.take().await
    }

    /// Re-signs `snippet` with capture `id`'s key and serves it.
    ///
    /// # Errors
    ///
    /// There's no capture `id`, or the snippet is over its bounds.
    pub fn update_capture(
        &self,
        id: &TransferId,
        mut snippet: ListingSnippet,
    ) -> Result<(), VideoError> {
        snippet::check(&snippet)?;
        let s = &self.shared;
        let (key, swarm_id) = s
            .captures
            .borrow()
            .get(id)
            .map(|c| (c.key.clone(), c.swarm_id.clone()))
            .ok_or(VideoError::CaptureNotFound)?;
        snippet.sign_message(&key)?;
        s.directory.push_snippet(&swarm_id, snippet)?;
        Ok(())
    }

    /// Appends `data` to capture `id`'s record in progress, and ends the
    /// record if `segment_end`. Resolves once written; see the
    /// [module docs](self).
    ///
    /// # Errors
    ///
    /// There's no capture `id` (or it closed first),
    /// [`VideoError::RecordDropped`] when the record's end is refused and
    /// the record dropped, or [`VideoError::StreamWrite`] once a swarm
    /// write failed.
    pub async fn append_capture(
        &self,
        id: &TransferId,
        data: Vec<u8>,
        segment_end: bool,
    ) -> Result<(), VideoError> {
        let appends = self
            .shared
            .captures
            .borrow()
            .get(id)
            .map(|c| c.appends.clone())
            .ok_or(VideoError::CaptureNotFound)?;
        let reply = Slot::new();
        let id = appends
            .push(data, segment_end, reply.clone())
            .ok_or(VideoError::CaptureNotFound)?;
        let _queued = Queued {
            appends: &appends,
            id,
        };
        reply.take().await
    }

    /// Closes capture `id`; see the [module docs](self).
    pub async fn close_capture(&self, id: &TransferId) -> Result<(), VideoError> {
        let s = &self.shared;
        let swarm = s.end_capture(id).ok_or(VideoError::CaptureNotFound)?;
        s.remove_host_swarm(swarm).await;
        Ok(())
    }

    /// The captures the node runs, in id order.
    pub fn captures(&self) -> Vec<CaptureInfo> {
        self.shared
            .captures
            .borrow()
            .iter()
            .map(|(id, c)| CaptureInfo {
                id: *id,
                swarm_uri: c.swarm_uri.clone(),
                mime_type: c.mime_type.clone(),
                listings: c
                    .keepers
                    .iter()
                    .map(|(k, keeper)| (k.clone(), keeper.listing_id()))
                    .collect(),
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capture_uri_carries_its_options() {
        let o = capture_options();
        let uri = Uri::new(SwarmId::new(&[0; 32]), o.uri_options()).to_string();
        assert!(
            uri.ends_with("&x.im=2&x.hf=4&x.sa=2&x.cs=1024&x.ps=32&x.sc=1"),
            "{uri}"
        );
        assert_eq!(o.live_window, 32 * 1024);
    }

    #[test]
    fn mime_types_are_short_printable_ascii() {
        for ok in ["video/webm;codecs=vp9,opus", "a", &"x".repeat(256)] {
            assert_eq!(check_mime(ok, 256), Ok(()), "{ok}");
        }
        for bad in ["", "video/webm\n", "vidéo", "a\u{7f}", &"x".repeat(257)] {
            assert_eq!(check_mime(bad, 256), Err(VideoError::MimeType), "{bad}");
        }
    }
}
