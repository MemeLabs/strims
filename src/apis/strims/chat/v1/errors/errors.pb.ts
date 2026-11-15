// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only


/**
 * Chat's error codes, 5000-5999: a chat server's refusals, which a node
 * relays to its UI as the server sent them, a whisper's refusal, the node's
 * own failures to reach a server, and the frontend's.
 */
export enum ErrorCode {
  /** No code: the message alone says what failed. */
  UNDEFINED = 0,
  /** The caller already posted the running combo's emote. */
  COMBO_DUPLICATE = 5000,
  /** Mute and unmute need the server's owner or one of its admins. */
  NOT_MODERATOR = 5001,
  /** The caller's mute hasn't expired. */
  MUTED = 5002,
  /** The message is over the server's bound. */
  MESSAGE_TOO_LONG = 5003,
  /** The caller is over its send rate. */
  TOO_FAST = 5004,
  /** A key, alias or mute message of the request is over its bound. */
  TOO_LARGE = 5005,
  /** The node doesn't run the network, or the profile has no such network. */
  NETWORK_NOT_FOUND = 5006,
  /**
   * The node hasn't joined the network, or its membership isn't trusted
   * yet.
   */
  NOT_JOINED = 5007,
  /** The node runs or reads the server on another network. */
  OTHER_NETWORK = 5008,
  /**
   * No address for the server, the server stopped, or the profile has no
   * server with the id.
   */
  SERVER_NOT_FOUND = 5009,
  /** The node reads as many chat rooms as it may. */
  TOO_MANY_ROOMS = 5010,
  /** The room has as many streams open as it may. */
  TOO_MANY_STREAMS = 5011,
  /** A call came without the caller's certificate. */
  NO_CALLER_CERTIFICATE = 5012,
  /** The recipient didn't take the whisper. It says no more than that. */
  WHISPER_REFUSED = 5013,
  /**
   * The profile has no such record (an emote, a modifier, a tag, a whisper
   * thread).
   */
  NOT_FOUND = 5014,
  /** A duration didn't parse, or is negative. */
  INVALID_DURATION = 5015,
  /** A name is empty. */
  NAME_EMPTY = 5016,
  /** A name is over its bound. */
  NAME_TOO_LONG = 5017,
  /** A record is over its bound. */
  RECORD_TOO_LARGE = 5018,
  /** An alias is over its bound. */
  ALIAS_TOO_LONG = 5019,
  /** A tag color is over its bound. */
  TAG_COLOR_TOO_LONG = 5020,
  /** A record has more labels than it may. */
  TOO_MANY_LABELS = 5021,
  /** A modifier's proc chance isn't from 0 to 1. */
  INVALID_PROC_CHANCE = 5022,
  /** A request names a part of a server's bundle that isn't one. */
  INVALID_PART = 5023,
  /** The server holds as many records of the kind as it may. */
  TOO_MANY_RECORDS = 5024,
  /** A whisper names no recipient. */
  WHISPER_SUBJECT_UNDEFINED = 5025,
  /** A certificate didn't decode. */
  INVALID_CERTIFICATE = 5026,
  /** A whisper's recipient is the sender. */
  WHISPER_TO_SELF = 5027,
  /** As many whispers await delivery as may. */
  TOO_MANY_WHISPERS_PENDING = 5028,
}

