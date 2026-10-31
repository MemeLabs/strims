// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { HlsApi, HlsLease, HlsRequest, RENEW_MS, hlsServed, openHls, reachable } from "./hls";

const REQ: HlsRequest = {
  swarmUri: "magnet:?xt=urn:ppspp:abc",
  networkKeys: [new Uint8Array([1])],
  mimeType: 'video/x.strims-units;codecs="avc1.64001f,Opus"',
};

// A node's HLS calls: whether it serves HLS, and the URL it answers.
const api = (supported: boolean, enabled: boolean, url: string | Error) => {
  const opens: unknown[] = [];
  const a = {
    isSupported: () => Promise.resolve({ supported }),
    getConfig: () => Promise.resolve({ config: { enabled, publicServerAddr: "" } }),
    openStream: (req: unknown) => {
      opens.push(req);
      return url instanceof Error ? Promise.reject(url) : Promise.resolve({ playlistUrl: url });
    },
  };
  return { api: a, opens };
};

describe("hls", () => {
  it("a node serves HLS when its driver supports it and its config enables it", async () => {
    assert.ok(await hlsServed(api(true, true, "").api));
    assert.ok(!(await hlsServed(api(false, true, "").api)));
    assert.ok(!(await hlsServed(api(true, false, "").api)));
    const failing = {
      isSupported: () => Promise.reject(new Error("closed")),
      getConfig: () => Promise.reject(new Error("closed")),
    };
    assert.ok(!(await hlsServed(failing as unknown as HlsApi)));
  });

  it("an open answers the playlist URL, or null when the node refuses", async () => {
    const ok = api(true, true, "https://node.example/hls/a/b/index.m3u8");
    assert.strictEqual(await openHls(ok.api, REQ), "https://node.example/hls/a/b/index.m3u8");
    assert.deepStrictEqual(ok.opens, [REQ]);
    assert.strictEqual(await openHls(api(true, true, new Error("HLS disabled")).api, REQ), null);
    assert.strictEqual(await openHls(api(true, true, "").api, REQ), null);
  });

  it("a URL is reachable from other devices unless it names a loopback host", () => {
    for (const url of [
      "http://10.0.0.134:8082/hls/a/b/index.m3u8",
      "https://cdn.strims.gg/hls/a/b/index.m3u8",
      "http://[2001:db8::1]:8082/x",
      "http://node.local:8082/x",
    ]) {
      assert.ok(reachable(url), url);
    }
    for (const url of [
      "http://127.0.0.1:8082/hls/a/b/index.m3u8",
      "http://127.5.0.1/x",
      "http://localhost:8082/x",
      "http://app.localhost/x",
      "http://[::1]:8082/x",
      "ftp://10.0.0.134/x",
      "not a url",
    ]) {
      assert.ok(!reachable(url), url);
    }
  });

  it("a lease opens the stream again every RENEW_MS until closed", async () => {
    const { api: a, opens } = api(true, true, "https://node.example/x.m3u8");
    let tick: () => void = null;
    let cleared: unknown = null;
    const lease = new HlsLease(a, REQ, {
      setInterval: (f, ms) => {
        assert.strictEqual(ms, RENEW_MS);
        tick = f;
        return 7;
      },
      clearInterval: (id) => (cleared = id),
    });
    assert.strictEqual(opens.length, 0);
    tick();
    tick();
    await Promise.resolve();
    assert.strictEqual(opens.length, 2);
    lease.close();
    assert.strictEqual(cleared, 7);
  });
});
