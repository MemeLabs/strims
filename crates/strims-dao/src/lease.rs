// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Leases: locks kept in the store, so that among the processes or
//! browser tabs sharing a store, one at a time runs a task such as
//! replication.
//!
//! A lease record holds its holder's random token and an expiry. A holder
//! refreshes it every [`REFRESH`]; anyone may take a lease whose expiry
//! has passed. Contenders check again after a random delay between
//! [`RECHECK_MIN`] and [`RECHECK_MAX`].

use rand_core::Rng;
use strims_base::rng::DetRng;
use strims_base::time::{Duration, Time};
use strims_proto::dao::v1::Mutex;

use crate::key::{Namespace, Part};

/// How long a lease lasts without a refresh.
pub const TTL: Duration = Duration::from_secs(10);
/// How often a holder refreshes its lease.
pub const REFRESH: Duration = Duration::from_secs(5);
/// The shortest wait before a contender checks again.
pub const RECHECK_MIN: Duration = Duration::from_secs(5);
/// The longest wait before a contender checks again.
pub const RECHECK_MAX: Duration = Duration::from_secs(15);

const NS: Namespace = Namespace(1001);

/// A holder's identity.
pub type Token = [u8; 16];

/// Another holder's lease hasn't expired.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("lease is held")]
pub struct Busy;

/// A new holder identity.
pub fn new_token(rng: &mut DetRng) -> Token {
    let mut t = Token::default();
    rng.fill_bytes(&mut t);
    t
}

/// The key of the lease named by `parts`.
pub fn key(parts: &[&str]) -> String {
    let parts: Vec<Part<'_>> = parts.iter().map(|p| Part::Str(p)).collect();
    NS.format(&parts)
}

/// The record that takes or refreshes the lease for `token` at `now`,
/// given the stored record.
pub fn try_acquire(existing: Option<&Mutex>, token: &Token, now: Time) -> Result<Mutex, Busy> {
    if let Some(m) = existing
        && m.eol > now.unix_nanos()
        && m.token.as_slice() != token
    {
        return Err(Busy);
    }
    Ok(Mutex {
        eol: (now + TTL).unix_nanos(),
        token: token.to_vec(),
    })
}

/// Whether the stored record is `token`'s.
pub fn is_holder(existing: &Mutex, token: &Token) -> bool {
    existing.token.as_slice() == token
}

/// A contender's wait before it checks again.
pub fn recheck_delay(rng: &mut DetRng) -> Duration {
    let span = (RECHECK_MAX - RECHECK_MIN).as_millis().max(1) as u64;
    RECHECK_MIN + Duration::from_millis((rng.next_u64() % span) as i64)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use strims_base::rng::det_rng;

    use super::*;

    #[test]
    fn leases_expire_and_refresh() {
        let mut rng = det_rng(1);
        let (a, b) = (new_token(&mut rng), new_token(&mut rng));
        let t0 = Time::from_unix_millis(1_000);

        let held = try_acquire(None, &a, t0).unwrap();
        assert_eq!(held.eol, (t0 + TTL).unix_nanos());
        assert!(is_holder(&held, &a));
        assert_eq!(try_acquire(Some(&held), &b, t0 + REFRESH), Err(Busy));

        let refreshed = try_acquire(Some(&held), &a, t0 + REFRESH).unwrap();
        assert_eq!(refreshed.eol, (t0 + REFRESH + TTL).unix_nanos());
        assert_eq!(try_acquire(Some(&refreshed), &b, t0 + TTL), Err(Busy));

        let taken = try_acquire(Some(&refreshed), &b, t0 + REFRESH + TTL).unwrap();
        assert!(is_holder(&taken, &b));
        assert!(!is_holder(&taken, &a));
    }

    #[test]
    fn keys_join_their_parts() {
        assert_eq!(key(&["replication"]), "rt:replication");
        assert_eq!(key(&["dao", "upgrade"]), "rt:dao:upgrade");
    }

    #[test]
    fn recheck_delays_stay_in_range() {
        let mut rng = det_rng(2);
        for _ in 0..1000 {
            let d = recheck_delay(&mut rng);
            assert!(d >= RECHECK_MIN && d < RECHECK_MAX);
        }
    }
}
