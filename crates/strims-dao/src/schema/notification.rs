// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Notifications for the profile's user, such as the outcome of a
//! certificate renewal.
//!
//! Notifications are local to the device, so they don't replicate. A
//! notification about a subject replaces the last one about it, and the
//! table holds at most [`MAX_NOTIFICATIONS`]; [`dispatch`] keeps both.
//! Deleting a network deletes the notifications about it.

use strims_base::time::Time;
use strims_kv::Kv;
use strims_proto::notification::v1::Notification;
use strims_proto::notification::v1::notification::subject::Model;
use strims_proto::notification::v1::notification::{Status, Subject};

use crate::error::Error;
use crate::events::Record;
use crate::key::Namespace;
use crate::store::WriteTx;
use crate::table::Table;

/// The most notifications [`dispatch`] leaves in the table.
pub const MAX_NOTIFICATIONS: usize = 64;

/// The user's notifications.
pub struct Notifications;

impl Table for Notifications {
    type Record = Notification;
    const NS: Namespace = Namespace(5001);

    fn id(r: &Notification) -> u64 {
        r.id
    }

    fn observe(r: &Notification) -> Option<Record> {
        Some(Record::Notification(r.clone()))
    }
}

/// A notification `id` raised at `now`, about `subject` if given.
pub fn new_notification(
    id: u64,
    now: Time,
    status: Status,
    title: &str,
    message: &str,
    subject: Option<Subject>,
) -> Notification {
    Notification {
        id,
        created_at: now.unix_millis(),
        status: status.into(),
        title: title.to_owned(),
        message: message.to_owned(),
        subject,
        error_code: 0,
    }
}

/// The subject naming the network `id`.
pub fn network_subject(id: u64) -> Subject {
    Subject {
        model: Model::NotificationSubjectModelNetwork.into(),
        id,
    }
}

/// Stores `n`. It replaces any notification with its id or its subject,
/// and the oldest others, by creation time, go until the table holds
/// [`MAX_NOTIFICATIONS`]. The deletions' events precede `n`'s.
pub async fn dispatch<K: Kv>(tx: &mut WriteTx<K>, n: Notification) -> Result<Notification, Error> {
    let mut kept = Vec::new();
    for p in Notifications::get_all(tx).await? {
        if p.id == n.id || (n.subject.is_some() && p.subject == n.subject) {
            Notifications::delete(tx, p.id).await?;
        } else {
            kept.push((p.created_at, p.id));
        }
    }
    kept.sort_unstable();
    let excess = (kept.len() + 1).saturating_sub(MAX_NOTIFICATIONS);
    for (_, id) in kept.into_iter().take(excess) {
        Notifications::delete(tx, id).await?;
    }
    Notifications::insert(tx, n).await
}

/// Deletes the notifications about `subject`.
pub async fn delete_for_subject<K: Kv>(tx: &mut WriteTx<K>, subject: Subject) -> Result<(), Error> {
    for p in Notifications::get_all(tx).await? {
        if p.subject == Some(subject) {
            Notifications::delete(tx, p.id).await?;
        }
    }
    Ok(())
}
