// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The profile, its devices and its id ranges.

use futures_util::future::LocalBoxFuture;
use strims_kv::Kv;
use strims_proto::dao::v1::VersionVector;
use strims_proto::profile::v1::{self as profile, Device, ProfileId};
use strims_proto::replication::v1::Checkpoint;

use crate::error::Error;
use crate::events::Record;
use crate::ids;
use crate::key::Namespace;
use crate::schema::system::Checkpoints;
use crate::store::WriteTx;
use crate::table::{Replication, Singleton, Table};

/// The profile: its id, name, key and this device's id.
pub struct Profile;

impl Singleton for Profile {
    type Record = profile::Profile;
    const NS: Namespace = Namespace(2001);
}

/// The id ranges this device allocates record ids from. Each device holds
/// its own ranges, so the record doesn't replicate.
pub struct ProfileIds;

impl Singleton for ProfileIds {
    type Record = ProfileId;
    const NS: Namespace = Namespace(2002);

    fn default_value() -> Option<ProfileId> {
        Some(ids::initial_ranges())
    }
}

/// The profile's devices. Each device is a replica, identified by its
/// device id; deleting a device marks its checkpoint deleted, and a
/// deleted replica can't replicate again.
pub struct Devices;

impl Table for Devices {
    type Record = Device;
    const NS: Namespace = Namespace(2003);
    const REPLICATION: Option<Replication<Device>> = Some(Replication::DEFAULT);

    fn id(r: &Device) -> u64 {
        r.id
    }

    fn observe(r: &Device) -> Option<Record> {
        Some(Record::Device(r.clone()))
    }

    fn on_delete<'a, K: Kv>(
        tx: &'a mut WriteTx<K>,
        p: &'a Device,
    ) -> LocalBoxFuture<'a, Result<(), Error>> {
        Box::pin(async move {
            Checkpoints::upsert(
                tx,
                Checkpoint {
                    id: p.id,
                    version: Some(VersionVector::default()),
                    deleted: true,
                },
            )
            .await?;
            Ok(())
        })
    }
}
