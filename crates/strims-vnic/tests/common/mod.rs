// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! A minimal synchronous driver for tests: moves bytes between hosts and
//! records what they deliver.

#![allow(
    dead_code,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing
)]

use strims_base::rng::det_rng;
use strims_base::time::Time;
use strims_crypto::key_from_seed;
use strims_vnic::{Event, Host, HostConfig, PeerKey};

/// The routing tag tests use: the port a handler was set for.
pub type Tag = u32;

pub struct Node {
    pub host: Host<Tag>,
    /// (peer, port, handler, body) in delivery order.
    pub recv: Vec<(PeerKey, u16, Tag, Vec<u8>)>,
    pub events: Vec<Event>,
}

impl Node {
    pub fn new(seed: u8, label: &str) -> Node {
        let config = HostConfig {
            label: label.to_owned(),
            ..HostConfig::default()
        };
        Node {
            host: Host::new(key_from_seed(&[seed; 32]), config, det_rng(u64::from(seed))).unwrap(),
            recv: Vec::new(),
            events: Vec::new(),
        }
    }

    /// Feeds `data` to `link`, handling `PeerAdded` with `on_peer` before the
    /// rest of the input, as a driver must.
    pub fn input(
        &mut self,
        link: PeerKey,
        now: Time,
        data: &mut [u8],
        on_peer: &mut dyn FnMut(&mut Host<Tag>, PeerKey),
    ) -> Result<(), strims_vnic::Error> {
        let mut off = 0;
        while off < data.len() {
            let Node { host, recv, events } = self;
            let res = host.handle_link_input(link, now, &mut data[off..], &mut |f| {
                recv.push((f.peer, f.port, f.handler, f.body.to_vec()));
            });
            while let Some(e) = host.poll_event() {
                if let Event::PeerAdded { peer, .. } = &e {
                    on_peer(host, *peer);
                }
                events.push(e);
            }
            off += res?;
        }
        Ok(())
    }

    /// Takes everything pending on `link`.
    pub fn take_transmit(&mut self, link: PeerKey) -> Vec<u8> {
        let mut out = Vec::new();
        loop {
            let Ok(data) = self.host.poll_link_transmit(link) else {
                return out;
            };
            if data.is_empty() {
                return out;
            }
            let n = data.len();
            out.extend_from_slice(data);
            self.host.advance_link_transmit(link, n).unwrap();
        }
    }
}

/// Moves bytes both ways between `a`'s link `la` and `b`'s link `lb` in
/// chunks of `chunk` bytes until neither has anything to send.
pub fn pump(
    a: &mut Node,
    la: PeerKey,
    b: &mut Node,
    lb: PeerKey,
    now: Time,
    chunk: usize,
    on_peer: &mut dyn FnMut(&mut Host<Tag>, PeerKey),
) {
    for _ in 0..1000 {
        let ab = a.take_transmit(la);
        let ba = b.take_transmit(lb);
        if ab.is_empty() && ba.is_empty() {
            return;
        }
        let mut ab = ab;
        for c in ab.chunks_mut(chunk) {
            if b.host.has_link(lb) {
                let _ = b.input(lb, now, c, on_peer);
            }
        }
        let mut ba = ba;
        for c in ba.chunks_mut(chunk) {
            if a.host.has_link(la) {
                let _ = a.input(la, now, c, on_peer);
            }
        }
    }
    panic!("pump did not settle");
}

pub fn now() -> Time {
    Time::from_unix_millis(1_700_000_000_000)
}
