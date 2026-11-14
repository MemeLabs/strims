// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

use std::fmt;

use crate::rpc;

/// An error a service can answer a call with: a code and a message, the
/// [`rpc::Error`] it sends.
///
/// The code is a value of its owner's `ErrorCode` enum, the one in a
/// `strims.<api>.errors` package. Each owner's codes lie in a range of
/// their own (the build script's `errors.rs` lists them), so a code names
/// one error across the API and an error relayed from another service keeps
/// its code. `0` is no code: the message alone says what failed.
///
/// Callers branch on codes, and the message is for people: it may change
/// where a code may not.
pub trait ErrorCoded: fmt::Display {
    /// The error's code.
    fn code(&self) -> i32;

    /// The RPC error that answers a call with this error.
    fn to_rpc(&self) -> rpc::Error {
        rpc::Error {
            message: self.to_string(),
            code: self.code(),
        }
    }
}
