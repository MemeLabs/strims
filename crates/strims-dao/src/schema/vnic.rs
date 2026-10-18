// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! This device's vnic config: its upload cap and its peer budget.
//!
//! The config stays on the device, since each device has its own link,
//! and its changes are observable.

use strims_proto::vnic::v1::Config;

use crate::events::Record;
use crate::key::Namespace;
use crate::table::Singleton;

/// The upload cap, in bytes per second, before a config is stored.
pub const DEFAULT_MAX_UPLOAD_BYTES_PER_SECOND: u64 = 1 << 40;

/// The peer budget before a config is stored.
pub const DEFAULT_MAX_PEERS: u32 = 25;

/// This device's vnic config. It defaults to
/// [`DEFAULT_MAX_UPLOAD_BYTES_PER_SECOND`] and [`DEFAULT_MAX_PEERS`].
pub struct VnicConfig;

impl Singleton for VnicConfig {
    type Record = Config;
    const NS: Namespace = Namespace(8001);

    fn default_value() -> Option<Config> {
        Some(Config {
            max_upload_bytes_per_second: DEFAULT_MAX_UPLOAD_BYTES_PER_SECOND,
            max_peers: DEFAULT_MAX_PEERS,
        })
    }

    fn observe(r: &Config) -> Option<Record> {
        Some(Record::VnicConfig(*r))
    }
}
