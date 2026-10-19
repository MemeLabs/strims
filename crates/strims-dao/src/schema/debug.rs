// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! This device's debug config: whether it reads the mock streams listed on
//! a network, and which network that is.
//!
//! The config stays on the device and its changes are observable. Deleting
//! the network it names clears the key ([`clear_network`]).

use strims_kv::Kv;
use strims_proto::debug::v1::Config;

use crate::error::Error;
use crate::events::Record;
use crate::key::Namespace;
use crate::store::WriteTx;
use crate::table::Singleton;

/// This device's debug config. It defaults to disabled, with no network.
pub struct DebugConfig;

impl Singleton for DebugConfig {
    type Record = Config;
    const NS: Namespace = Namespace(12001);

    fn default_value() -> Option<Config> {
        Some(Config::default())
    }

    fn observe(r: &Config) -> Option<Record> {
        Some(Record::DebugConfig(r.clone()))
    }
}

/// Clears the config's mock stream network key if it is `network_key`. It
/// writes nothing otherwise.
pub async fn clear_network<K: Kv>(tx: &mut WriteTx<K>, network_key: &[u8]) -> Result<(), Error> {
    DebugConfig::transform(tx, |c| {
        if c.mock_stream_network_key == network_key {
            c.mock_stream_network_key.clear();
        }
        Ok(())
    })
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_debug_config_stays_local() {
        assert!(DebugConfig::REPLICATION.is_none());
    }
}
