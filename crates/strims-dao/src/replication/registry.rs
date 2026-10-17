// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The replicated record types, by namespace.

use std::cmp::Ordering;

use futures_util::future::LocalBoxFuture;
use prost::Message;
use strims_kv::Kv;
use strims_proto::dao::v1::VersionVector;
use strims_proto::replication::v1::Event;

use crate::error::Error;
use crate::key::Namespace;
use crate::schema::chat::{
    ChatUiConfig, ChatUiConfigHighlights, ChatUiConfigIgnores, ChatUiConfigTags, WhisperRecords,
    WhisperThreads,
};
use crate::schema::network::{BootstrapClients, Networks};
use crate::schema::profile::Devices;
use crate::store::{Read, WriteTx, get_record};
use crate::table::{Singleton, Table, write_row, write_singleton};
use crate::vv;

/// Every replicated namespace, in the order a dump lists them.
pub const REPLICATED: &[Namespace] = &[
    Devices::NS,
    Networks::NS,
    BootstrapClients::NS,
    ChatUiConfig::NS,
    ChatUiConfigHighlights::NS,
    ChatUiConfigTags::NS,
    ChatUiConfigIgnores::NS,
    WhisperThreads::NS,
    WhisperRecords::NS,
];

/// Runs `$on_table::<T>` or `$on_singleton::<S>` for the record type of
/// namespace `$ns`, or evaluates `$unknown`.
macro_rules! dispatch {
    ($ns:expr, $on_table:ident, $on_singleton:ident, ($($arg:expr),*), $unknown:expr) => {{
        let ns: Namespace = $ns;
        if ns == Devices::NS {
            $on_table::<Devices, _>($($arg),*)
        } else if ns == Networks::NS {
            $on_table::<Networks, _>($($arg),*)
        } else if ns == BootstrapClients::NS {
            $on_table::<BootstrapClients, _>($($arg),*)
        } else if ns == ChatUiConfig::NS {
            $on_singleton::<ChatUiConfig, _>($($arg),*)
        } else if ns == ChatUiConfigHighlights::NS {
            $on_table::<ChatUiConfigHighlights, _>($($arg),*)
        } else if ns == ChatUiConfigTags::NS {
            $on_table::<ChatUiConfigTags, _>($($arg),*)
        } else if ns == ChatUiConfigIgnores::NS {
            $on_table::<ChatUiConfigIgnores, _>($($arg),*)
        } else if ns == WhisperThreads::NS {
            $on_table::<WhisperThreads, _>($($arg),*)
        } else if ns == WhisperRecords::NS {
            $on_table::<WhisperRecords, _>($($arg),*)
        } else {
            $unknown
        }
    }};
}

/// Applies an event from another replica. Events of unknown namespaces
/// are skipped.
pub(crate) fn apply<'a, K: Kv>(
    tx: &'a mut WriteTx<K>,
    e: &'a Event,
) -> LocalBoxFuture<'a, Result<(), Error>> {
    dispatch!(
        Namespace(e.namespace),
        apply_table,
        apply_singleton,
        (tx, e),
        Box::pin(async { Ok(()) })
    )
}

/// The current state of record `id` of namespace `ns`, as an event.
/// `None` if the record doesn't exist or the namespace is unknown.
pub(crate) fn load<'a, R: Read>(
    tx: &'a mut R,
    ns: Namespace,
    id: u64,
) -> LocalBoxFuture<'a, Result<Option<Event>, Error>> {
    dispatch!(
        ns,
        load_table,
        load_singleton,
        (tx, id),
        Box::pin(async { Ok(None) })
    )
}

/// Every replicated record, as events.
pub(crate) async fn dump<R: Read>(tx: &mut R) -> Result<Vec<Event>, Error> {
    let mut out = Vec::new();
    for &ns in REPLICATED {
        out.extend(
            dispatch!(
                ns,
                dump_table,
                dump_singleton,
                (&mut *tx),
                Box::pin(async { Ok(Vec::new()) })
            )
            .await?,
        );
    }
    Ok(out)
}

fn decode_versioned<R: Message + Default>(
    e: &Event,
    version_mut: fn(&mut R) -> &mut Option<VersionVector>,
) -> Result<R, Error> {
    let mut r = R::decode(e.record.as_slice())?;
    let v = version_mut(&mut r);
    if v.is_none() {
        *v = e.version.clone();
    }
    Ok(r)
}

fn version<R>(r: &R, f: fn(&R) -> Option<&VersionVector>) -> VersionVector {
    f(r).cloned().unwrap_or_default()
}

fn apply_table<'a, T: Table, K: Kv>(
    tx: &'a mut WriteTx<K>,
    e: &'a Event,
) -> LocalBoxFuture<'a, Result<(), Error>> {
    Box::pin(async move {
        let Some(rep) = T::REPLICATION else {
            return Ok(());
        };
        if e.delete {
            T::delete(tx, e.id).await?;
            return Ok(());
        }
        let next = decode_versioned::<T::Record>(e, rep.version_mut)?;
        if T::id(&next) != e.id {
            return Err(Error::Invalid("replication event id"));
        }
        let local = T::get(tx, e.id).await?;
        let merged = match &local {
            None => next,
            Some(local) => {
                match vv::compare(&version(local, rep.version), &version(&next, rep.version)) {
                    None => (rep.resolve)(next, local),
                    Some(Ordering::Less) => (rep.merge)(next, local),
                    Some(_) => return Ok(()),
                }
            }
        };
        if local.as_ref() == Some(&merged) {
            return Ok(());
        }
        match write_row::<T, K>(tx, merged, local).await {
            Ok(_) | Err(Error::Superseded) => Ok(()),
            Err(e) => Err(e),
        }
    })
}

fn apply_singleton<'a, S: Singleton, K: Kv>(
    tx: &'a mut WriteTx<K>,
    e: &'a Event,
) -> LocalBoxFuture<'a, Result<(), Error>> {
    Box::pin(async move {
        let Some(rep) = S::REPLICATION else {
            return Ok(());
        };
        if e.delete || e.id != 0 {
            return Err(Error::Invalid("replication event"));
        }
        let next = decode_versioned::<S::Record>(e, rep.version_mut)?;
        let local: Option<S::Record> = get_record(tx, &S::NS.key()).await?;
        let merged = match &local {
            None => next,
            Some(local) => {
                match vv::compare(&version(local, rep.version), &version(&next, rep.version)) {
                    None => (rep.resolve)(next, local),
                    Some(Ordering::Less) => (rep.merge)(next, local),
                    Some(_) => return Ok(()),
                }
            }
        };
        if local.as_ref() == Some(&merged) {
            return Ok(());
        }
        write_singleton::<S, K>(tx, merged, local.as_ref()).await?;
        Ok(())
    })
}

/// Record `r` as an event. A record sent to another replica always
/// carries a version, if only an empty one, since receivers compare it.
fn event_of<R: Clone + Message>(
    ns: Namespace,
    id: u64,
    mut r: R,
    rep: &crate::table::Replication<R>,
) -> Event {
    let version = (rep.version_mut)(&mut r)
        .get_or_insert_with(VersionVector::default)
        .clone();
    Event {
        namespace: ns.0,
        id,
        version: Some(version),
        delete: false,
        record: (rep.extract)(r).encode_to_vec(),
    }
}

fn load_table<'a, T: Table, R: Read>(
    tx: &'a mut R,
    id: u64,
) -> LocalBoxFuture<'a, Result<Option<Event>, Error>> {
    Box::pin(async move {
        let Some(rep) = T::REPLICATION else {
            return Ok(None);
        };
        Ok(T::get(tx, id).await?.map(|r| event_of(T::NS, id, r, &rep)))
    })
}

fn load_singleton<'a, S: Singleton, R: Read>(
    tx: &'a mut R,
    _id: u64,
) -> LocalBoxFuture<'a, Result<Option<Event>, Error>> {
    Box::pin(async move {
        let Some(rep) = S::REPLICATION else {
            return Ok(None);
        };
        let r: Option<S::Record> = get_record(tx, &S::NS.key()).await?;
        Ok(r.map(|r| event_of(S::NS, 0, r, &rep)))
    })
}

fn dump_table<'a, T: Table, R: Read>(
    tx: &'a mut R,
) -> LocalBoxFuture<'a, Result<Vec<Event>, Error>> {
    Box::pin(async move {
        let Some(rep) = T::REPLICATION else {
            return Ok(Vec::new());
        };
        let rows = T::get_all(tx).await?;
        Ok(rows
            .into_iter()
            .map(|r| event_of(T::NS, T::id(&r), r, &rep))
            .collect())
    })
}

fn dump_singleton<'a, S: Singleton, R: Read>(
    tx: &'a mut R,
) -> LocalBoxFuture<'a, Result<Vec<Event>, Error>> {
    Box::pin(async move {
        let Some(rep) = S::REPLICATION else {
            return Ok(Vec::new());
        };
        let r: Option<S::Record> = get_record(tx, &S::NS.key()).await?;
        Ok(r.into_iter().map(|r| event_of(S::NS, 0, r, &rep)).collect())
    })
}
