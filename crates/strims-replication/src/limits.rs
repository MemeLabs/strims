// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

//! Bounds on what a peer's requests can carry. A request over a bound is
//! refused whole.

use strims_proto::replication::v1::{EventLog, PeerBootstrapRequest, PeerSyncRequest};

/// The most logs in one Sync.
pub const MAX_SYNC_LOGS: usize = 1024;
/// The most events in one log.
pub const MAX_LOG_EVENTS: usize = 4096;
/// The most events in one request.
pub const MAX_REQUEST_EVENTS: usize = 16 * 1024;
/// The most logs in one Bootstrap.
pub const MAX_BOOTSTRAP_LOGS: usize = 4096;
/// The most checkpoints in one request.
pub const MAX_CHECKPOINTS: usize = strims_dao::replication::MAX_REPLICAS;
/// The most replicas a version vector in a request names.
pub const MAX_VERSION_ENTRIES: usize = strims_dao::replication::MAX_REPLICAS;

/// A request over a bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[error("request exceeds {0}")]
pub struct OverLimit(pub &'static str);

fn check_log(l: &EventLog) -> Result<usize, OverLimit> {
    if l.events.len() > MAX_LOG_EVENTS {
        return Err(OverLimit("events per log"));
    }
    let versions = l
        .checkpoint
        .iter()
        .filter_map(|c| c.version.as_ref())
        .chain(l.events.iter().filter_map(|e| e.version.as_ref()));
    for v in versions {
        if v.value.len() > MAX_VERSION_ENTRIES {
            return Err(OverLimit("version entries"));
        }
    }
    Ok(l.events.len())
}

/// Checks a Sync request.
pub fn check_sync(req: &PeerSyncRequest) -> Result<(), OverLimit> {
    if req.logs.len() > MAX_SYNC_LOGS {
        return Err(OverLimit("logs per sync"));
    }
    let mut events = 0usize;
    for l in &req.logs {
        events = events.saturating_add(check_log(l)?);
    }
    if events > MAX_REQUEST_EVENTS {
        return Err(OverLimit("events per request"));
    }
    Ok(())
}

/// Checks a Bootstrap request.
pub fn check_bootstrap(req: &PeerBootstrapRequest) -> Result<(), OverLimit> {
    if req.logs.len() > MAX_BOOTSTRAP_LOGS {
        return Err(OverLimit("logs per bootstrap"));
    }
    if req.checkpoints.len() > MAX_CHECKPOINTS {
        return Err(OverLimit("checkpoints per request"));
    }
    let mut events = req.events.len();
    for l in &req.logs {
        events = events.saturating_add(check_log(l)?);
    }
    if events > MAX_REQUEST_EVENTS {
        return Err(OverLimit("events per request"));
    }
    let versions = req
        .events
        .iter()
        .filter_map(|e| e.version.as_ref())
        .chain(req.checkpoints.iter().filter_map(|c| c.version.as_ref()));
    for v in versions {
        if v.value.len() > MAX_VERSION_ENTRIES {
            return Err(OverLimit("version entries"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use strims_proto::replication::v1::Event;

    use super::*;

    #[test]
    fn oversized_requests_are_refused() {
        let log = |n: usize| EventLog {
            events: vec![Event::default(); n],
            ..Default::default()
        };
        assert_eq!(
            check_sync(&PeerSyncRequest {
                logs: vec![log(10); 3]
            }),
            Ok(())
        );
        assert!(
            check_sync(&PeerSyncRequest {
                logs: vec![log(0); MAX_SYNC_LOGS + 1]
            })
            .is_err()
        );
        assert!(
            check_sync(&PeerSyncRequest {
                logs: vec![log(MAX_LOG_EVENTS + 1)]
            })
            .is_err()
        );
        assert!(
            check_sync(&PeerSyncRequest {
                logs: vec![log(MAX_LOG_EVENTS); 5]
            })
            .is_err()
        );
        let req = PeerBootstrapRequest {
            events: vec![Event::default(); MAX_REQUEST_EVENTS + 1],
            ..Default::default()
        };
        assert!(check_bootstrap(&req).is_err());
    }
}
