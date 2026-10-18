// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! The record types of a profile store.
//!
//! Namespace numbers are part of the stored format and of the replication
//! protocol (`Event.namespace`), so they never change. Top-level areas
//! take multiples of 1000, and their record types count up from there.

pub mod chat;
pub mod directory;
pub mod network;
pub mod notification;
pub mod profile;
pub mod system;
