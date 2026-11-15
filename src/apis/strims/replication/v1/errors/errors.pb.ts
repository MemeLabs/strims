// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only


/**
 * Replication's error codes, 9000-9999: a peer's refusals and the
 * reasons replication to a peer stopped.
 */
export enum ErrorCode {
  /** No code: the message alone says what failed. */
  UNDEFINED = 0,
  /** The peer hasn't opened replication. */
  NOT_OPEN = 9000,
  /** The replica is a deleted device's. */
  REPLICA_DELETED = 9001,
  /** The peer's store has another format version. */
  UNSUPPORTED_STORE_VERSION = 9002,
  /** A replica id is 0, or this replica's. */
  INVALID_REPLICA_ID = 9003,
  /** A request holds more than its bound. */
  OVER_LIMIT = 9004,
  /** The peer answered something malformed. */
  INVALID_ANSWER = 9005,
}

