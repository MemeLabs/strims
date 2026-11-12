// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The node's checks of the video settings' addresses
// (crates/strims-video/src/config.rs), so a form refuses what the node would
// refuse when it saves the config.

/** The longest address the node takes, in UTF-8 bytes. */
export const MAX_ADDR_BYTES = 256;

const encoder = new TextEncoder();

/** Whether `addr` fits in [`MAX_ADDR_BYTES`]. */
export const isAddrLength = (addr: string): boolean =>
  encoder.encode(addr).length <= MAX_ADDR_BYTES;

// A host as a URL can carry it: printable ASCII without the characters that
// would end a URL's authority or add userinfo.
const isHost = (host: string): boolean => /^[\x21-\x7e]+$/.test(host) && !/[/?#@]/.test(host);

const isPort = (port: string): boolean => /^\d+$/.test(port) && +port > 0 && +port <= 0xffff;

/** A listen address: a host, then `:` and a nonzero port (`[::]:1935`). */
export const isHostPort = (addr: string): boolean => {
  const i = addr.lastIndexOf(":");
  return i >= 0 && isHost(addr.slice(0, i)) && isPort(addr.slice(i + 1));
};

/**
 * A host with an optional port, as `rtmp://<addr>/live/<key>` takes it. A
 * trailing `:` and digits must be a valid port.
 */
export const isHostOptionalPort = (addr: string): boolean =>
  isHostPort(addr) || (isHost(addr) && !/:\d*$/.test(addr));

/** An `http://` or `https://` URL with a host. */
export const isHttpUrl = (addr: string): boolean => {
  if (!/^https?:\/\//i.test(addr)) {
    return false;
  }
  try {
    return new URL(addr).host !== "";
  } catch {
    return false;
  }
};
