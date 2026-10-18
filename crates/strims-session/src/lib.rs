// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Profile sessions.
//!
//! A [`Session`] is a signed-in profile on this device: its open store and
//! its profile record. The [`Manager`] opens at most one session per
//! profile, records this device's sign-in on the profile's device record,
//! and has the driver start the session's runtime (its node, replication
//! and services) through a [`SessionFactory`].
//!
//! A frontend connection starts with only [`AuthService`] bound. Signing
//! in or up starts or joins the profile's session and binds the session's
//! frontend services on the same connection. [`bind_frontends`] binds
//! those that need only the session's store: its profile and devices,
//! replication, and the user's notifications ([`NotificationService`]).

mod auth;
mod manager;
mod notification;
mod profile;

use strims_proto::service::Error as RpcError;

pub use auth::AuthService;
pub use manager::{Manager, Session, SessionFactory};
pub use notification::NotificationService;
pub use profile::{ProfileService, bind_frontends};

/// The RPC error a store error answers with.
pub(crate) fn rpc_error(e: strims_dao::Error) -> RpcError {
    RpcError::new(0, e.to_string())
}
