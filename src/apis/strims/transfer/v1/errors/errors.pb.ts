// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only


/** Transfers' error codes, 8000-8999. */
export enum ErrorCode {
  /** No code: the message alone says what failed. */
  UNDEFINED = 0,
  /** A transfer id isn't of the length transfer ids are. */
  INVALID_TRANSFER_ID = 8000,
  /** The node's transfers have stopped. */
  TRANSFERS_STOPPED = 8001,
}

