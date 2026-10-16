// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Errors returned by the node API.

/// Errors from [`crate::Node`] calls.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum NodeError {
    /// The node task has stopped.
    #[error("node stopped")]
    Stopped,
    #[error("io: {0}")]
    Io(String),
    /// The dial failed before or during the vnic handshake.
    #[error("dial failed: {0}")]
    Dial(String),
    #[error("invalid uri: {0}")]
    InvalidUri(String),
    /// No connected peer with this host id.
    #[error("unknown peer")]
    UnknownPeer,
    #[error("unknown swarm")]
    UnknownSwarm,
    /// The swarm writer or reader was closed (or its swarm removed), or the
    /// peer RPC pair was closed.
    #[error("closed")]
    Closed,
    #[error("unsupported: {0}")]
    Unsupported(&'static str),
    /// The peer's RPC channels were already taken.
    #[error("peer rpc already taken")]
    RpcTaken,
    /// A peer RPC channel received more than its buffer holds, so the pair
    /// was closed.
    #[error("peer rpc input overflowed")]
    RpcOverflow,
    #[error("config: {0}")]
    Config(String),
    #[error("vnic: {0}")]
    Vnic(#[from] strims_vnic::Error),
    #[error("vpn: {0}")]
    Vpn(#[from] strims_vpn::Error),
    #[error("ppspp: {0}")]
    Runner(#[from] strims_ppspp::RunnerError),
    #[error("swarm: {0}")]
    Swarm(#[from] strims_ppspp::SwarmError),
}

impl From<std::io::Error> for NodeError {
    fn from(e: std::io::Error) -> NodeError {
        NodeError::Io(e.to_string())
    }
}

/// Errors from [`crate::SwarmReader::read`]. Only `Closed` and `Stopped`
/// end the reader; after the others the next read continues.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReadError {
    /// The live window moved past unread data; the reader skipped
    /// `skipped` bytes to the window's tail.
    #[error("buffer underrun: skipped {skipped} bytes")]
    Underrun { skipped: u64 },
    /// The publisher reset the stream (a new epoch); reading restarts with
    /// the new stream.
    #[error("stream reset")]
    Reset,
    /// The swarm was removed or closed.
    #[error("closed")]
    Closed,
    /// The node task has stopped.
    #[error("node stopped")]
    Stopped,
    #[error("store: {0}")]
    Store(String),
}
