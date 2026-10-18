// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! This device's autoseed config and rules. A rule names a swarm by
//! network key, swarm id and salt; while autoseed is enabled, the device
//! seeds the swarms its rules name.
//!
//! Both stay on the device and are observable. Deleting a network deletes
//! the rules on it ([`delete_for_network`]).

use strims_kv::Kv;
use strims_proto::autoseed::v1::{Config, Rule};

use crate::error::Error;
use crate::events::Record;
use crate::key::Namespace;
use crate::store::WriteTx;
use crate::table::{Singleton, Table};

/// This device's autoseed config. It defaults to disabled.
pub struct AutoseedConfig;

impl Singleton for AutoseedConfig {
    type Record = Config;
    const NS: Namespace = Namespace(10001);

    fn default_value() -> Option<Config> {
        Some(Config::default())
    }

    fn observe(r: &Config) -> Option<Record> {
        Some(Record::AutoseedConfig(*r))
    }
}

/// This device's autoseed rules.
///
/// The table must stay without indexes: stores may hold rules that have
/// no index entries. Lookups other than by id scan the table, whose size
/// the autoseed frontend bounds.
pub struct AutoseedRules;

impl Table for AutoseedRules {
    type Record = Rule;
    const NS: Namespace = Namespace(10002);

    fn id(r: &Rule) -> u64 {
        r.id
    }

    fn observe(r: &Rule) -> Option<Record> {
        Some(Record::AutoseedRule(r.clone()))
    }
}

/// `rule` with a new id. It isn't stored.
pub async fn new_rule<K: Kv>(tx: &mut WriteTx<K>, rule: Rule) -> Result<Rule, Error> {
    let id = tx.generate_id().await?;
    Ok(Rule { id, ..rule })
}

/// Deletes the rules on the network with key `network_key`.
pub async fn delete_for_network<K: Kv>(
    tx: &mut WriteTx<K>,
    network_key: &[u8],
) -> Result<(), Error> {
    for r in AutoseedRules::get_all(tx).await? {
        if r.network_key == network_key {
            AutoseedRules::delete(tx, r.id).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn autoseed_records_stay_local() {
        assert!(AutoseedConfig::REPLICATION.is_none());
        assert!(AutoseedRules::REPLICATION.is_none());
        assert!(AutoseedRules::INDEXES.is_empty());
    }
}
