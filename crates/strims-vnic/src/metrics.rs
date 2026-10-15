// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Plain counters for the driver to export. Dials and handler errors are
//! counted where they happen: in the driver and in the owners.

/// Per-link transport counters. Reads count per `handle_link_input` call and
/// writes per `advance_link_transmit` call.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LinkMetrics {
    pub read_bytes: u64,
    pub read_count: u64,
    pub write_bytes: u64,
    pub write_count: u64,
}

/// Host-wide counters.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HostMetrics {
    /// Established peers.
    pub links_active: u64,
    pub frame_read_count: u64,
    pub frame_read_bytes: u64,
    /// Frames for a port with no handler, which are dropped.
    pub frame_handler_not_found_count: u64,
    /// Frames filled by owners in `poll_egress`.
    pub frame_write_count: u64,
    pub frame_write_bytes: u64,
    /// Keepalive frames queued and received. Received keepalives count only
    /// here.
    pub keepalive_write_count: u64,
    pub keepalive_read_count: u64,
    /// Links closed by the handshake deadline.
    pub handshake_timeout_count: u64,
    /// Links in the handshake closed to make room for newer ones.
    pub handshake_evict_count: u64,
}
