// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only


/**
 * Video's error codes, 6000-6999: channels, captures, streams, HLS egress
 * and autoseed.
 */
export enum ErrorCode {
  /** No code: the message alone says what failed. */
  UNDEFINED = 0,
  /** The profile has no channel with the id. */
  CHANNEL_NOT_FOUND = 6000,
  /** The profile has as many channels as it may. */
  TOO_MANY_CHANNELS = 6001,
  /** The key names none of the profile's networks. */
  NETWORK_NOT_FOUND = 6002,
  /**
   * The channel is another key holder's, or has no owner, so the profile
   * can't make its stream key.
   */
  NO_STREAM_KEY = 6003,
  /** The profile has no key to sign stream keys with. */
  NO_PROFILE_KEY = 6004,
  /**
   * A capture names no networks, or a capture or stream more than its
   * bound.
   */
  NETWORK_KEY_COUNT = 6005,
  /** A stream's network key is over its bound. */
  NETWORK_KEY_TOO_LONG = 6006,
  /** The node runs as many captures as it may. */
  TOO_MANY_CAPTURES = 6007,
  /** No capture of the node, or of the connection, has the id. */
  CAPTURE_NOT_FOUND = 6008,
  /**
   * A capture's decoder config or access unit is malformed or breaks the
   * capture's rules; the capture goes on.
   */
  INVALID_CAPTURE_MEDIA = 6009,
  /** A capture's append would take it over its rate. */
  CAPTURE_RATE = 6010,
  /** A capture's waiting appends hold as much as they may. */
  CAPTURE_BUSY = 6011,
  /** A swarm uri is over its bound. */
  URI_TOO_LONG = 6012,
  /** A swarm uri doesn't parse. */
  INVALID_SWARM_URI = 6013,
  /** A swarm option of the uri is outside the egress bounds or unsupported. */
  UNSUPPORTED_SWARM_OPTION = 6014,
  /** A stream's format isn't the unit format the node reads. */
  UNSUPPORTED_FORMAT = 6015,
  /** The node reads as many swarms for streams as it may. */
  TOO_MANY_SWARMS = 6016,
  /** The node has as many streams open as it may. */
  TOO_MANY_STREAMS = 6017,
  /** A stream's swarm uri names other options than the swarm's listing. */
  LISTING_OPTIONS = 6018,
  /**
   * The node reads the swarm with other options, which changed too
   * recently to change again.
   */
  REOPEN_TOO_SOON = 6019,
  /** The stream's swarm was removed, or couldn't be added. */
  STREAM_CLOSED = 6020,
  /** A swarm read failed. */
  STREAM_READ = 6021,
  /** A swarm write failed; the capture writes no more. */
  STREAM_WRITE = 6022,
  /** Signing a snippet failed. */
  SIGN = 6023,
  /** The node doesn't serve HLS. */
  HLS_NOT_SUPPORTED = 6024,
  /** The profile's HLS streams can't be routed. */
  HLS_UNAVAILABLE = 6025,
  /** The device's HLS egress config is disabled. */
  HLS_DISABLED = 6026,
  /** HLS egress has no server address. */
  HLS_NO_SERVER_ADDR = 6027,
  /** The node serves as many HLS streams as it may. */
  TOO_MANY_HLS_STREAMS = 6028,
  /** The node has no HLS stream of the transfer. */
  HLS_STREAM_NOT_FOUND = 6029,
  /** The device has no autoseed rule with the id. */
  AUTOSEED_RULE_NOT_FOUND = 6030,
  /** The device has as many autoseed rules as it may. */
  TOO_MANY_AUTOSEED_RULES = 6031,
  /** A field of an autoseed rule is empty or over its bound. */
  INVALID_AUTOSEED_RULE = 6032,
  /** A config's listen address is over its bound. */
  SERVER_ADDR_TOO_LONG = 6033,
  /** A config's listen address isn't host:port. */
  INVALID_SERVER_ADDR = 6034,
  /** A config's public address is over its bound. */
  PUBLIC_ADDR_TOO_LONG = 6035,
  /** A config's public address isn't an http:// or https:// url. */
  INVALID_PUBLIC_URL = 6036,
  /** An ingress config holds more service networks than it may. */
  TOO_MANY_SERVICE_NETWORKS = 6037,
  /** A service network key is over its bound. */
  SERVICE_NETWORK_KEY_TOO_LONG = 6038,
  /** The node runs a swarm with the capture's id already. */
  SWARM_ID_IN_USE = 6039,
}

