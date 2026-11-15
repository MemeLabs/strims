// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only


/**
 * The node's error codes, 3000-3999, which a service answers with when an
 * operation on its node fails.
 */
export enum ErrorCode {
  /** No code: the message alone says what failed. */
  UNDEFINED = 0,
  /** The node has stopped. */
  STOPPED = 3000,
  /** An io operation failed. */
  IO = 3001,
  /** A dial failed before or during the vnic handshake. */
  DIAL = 3002,
  /** The dialed link reached this node. */
  OWN_HOST = 3003,
  /** An address didn't parse. */
  INVALID_URI = 3004,
  /** No connected peer has the host id. */
  UNKNOWN_PEER = 3005,
  /** The node runs no swarm with the id. */
  UNKNOWN_SWARM = 3006,
  /**
   * The swarm writer or reader was closed, its swarm removed, or the peer
   * RPC pair closed.
   */
  CLOSED = 3007,
  /** The node doesn't support the operation. */
  UNSUPPORTED = 3008,
  /** The peer's RPC channels were already taken. */
  RPC_TAKEN = 3009,
  /** A peer RPC channel received more than its buffer holds. */
  RPC_OVERFLOW = 3010,
  /** The node's config is invalid. */
  CONFIG = 3011,
  /** The vnic failed. */
  VNIC = 3012,
  /** The vpn failed. */
  VPN = 3013,
  /** A certificate doesn't decode. */
  CERTIFICATE = 3014,
  /** The ppspp runner refused the operation. */
  RUNNER = 3015,
  /** The swarm refused the operation. */
  SWARM = 3016,
  /** A swarm reader fell behind the live window and skipped ahead. */
  UNDERRUN = 3017,
  /** The publisher reset the stream. */
  RESET = 3018,
  /** A swarm reader's store failed. */
  READ_STORE = 3019,
  /** A rewind left the reader where it was. */
  REWIND_REFUSED = 3020,
  /** Drawing the node's seed from entropy failed. */
  ENTROPY = 3021,
  /** The profile has no key to start its node with. */
  NO_PROFILE_KEY = 3022,
  /** The process runs another profile's node, and it runs one at a time. */
  PROFILE_RUNNING = 3023,
}

