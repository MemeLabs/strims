// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only


/**
 * The session's error codes, 7000-7999: sign-in, pairing, the profile's
 * devices and the vnic config.
 */
export enum ErrorCode {
  /** No code: the message alone says what failed. */
  UNDEFINED = 0,
  /** The frontend connection closed while signing in. */
  CONNECTION_CLOSED = 7000,
  /** A pairing token doesn't open. */
  INVALID_PAIRING_TOKEN = 7001,
  /** A sign-in names no credentials. */
  MISSING_CREDENTIALS = 7002,
  /** A new profile has no name. */
  NAME_REQUIRED = 7003,
  /** The device signed in can't delete itself. */
  CANNOT_DELETE_CURRENT_DEVICE = 7004,
  /** A request lacks its config. */
  CONFIG_MISSING = 7005,
}

