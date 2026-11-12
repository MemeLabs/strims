// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { MAX_ADDR_BYTES, isAddrLength, isHostOptionalPort, isHostPort, isHttpUrl } from "./address";

describe("address", () => {
  // crates/strims-video/src/config.rs: listen_addresses_are_hosts_and_ports.
  it("takes listen addresses that are hosts and ports", () => {
    for (const ok of ["127.0.0.1:1935", "0.0.0.0:1935", "[::]:1935", "a.b:65535", "localhost:1"]) {
      assert.ok(isHostPort(ok), ok);
    }
    for (const bad of [
      "",
      "127.0.0.1",
      ":1935",
      "host:",
      "host:+1935",
      "host:65536",
      "host:0",
      "a b:1935",
      "a/b:1935",
      "user@host:1935",
      "not an address",
    ]) {
      assert.ok(!isHostPort(bad), bad);
    }
  });

  it("takes public ingress addresses with or without a port", () => {
    for (const ok of ["ingress.strims.gg", "ingress.strims.gg:1935", "203.0.113.7", "[2001:db8::1]"]) {
      assert.ok(isHostOptionalPort(ok), ok);
    }
    for (const bad of [
      "",
      "rtmp://ingress.strims.gg",
      "ingress.strims.gg:",
      "ingress.strims.gg:0",
      "ingress.strims.gg:99999",
      "ingress strims",
      "not an address",
    ]) {
      assert.ok(!isHostOptionalPort(bad), bad);
    }
  });

  // crates/strims-video/src/config.rs: hls_public_addresses_are_empty_or_http_urls.
  it("takes HLS public addresses that are http URLs", () => {
    for (const ok of ["http://a", "https://cdn.strims.gg:8443/x", "HTTPS://A"]) {
      assert.ok(isHttpUrl(ok), ok);
    }
    for (const bad of ["cdn.strims.gg", "ftp://a", "http:/a", "é", "http://", "not an address"]) {
      assert.ok(!isHttpUrl(bad), bad);
    }
  });

  it("bounds addresses in UTF-8 bytes", () => {
    assert.ok(isAddrLength("h".repeat(MAX_ADDR_BYTES)));
    assert.ok(!isAddrLength("h".repeat(MAX_ADDR_BYTES + 1)));
    assert.ok(!isAddrLength("é".repeat(MAX_ADDR_BYTES / 2 + 1)));
  });
});
