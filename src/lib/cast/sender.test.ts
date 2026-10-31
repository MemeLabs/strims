// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  CAST_SDK_URL,
  CastContext,
  CastContextEvent,
  CastSdk,
  CastSender,
  CastSession,
  CastState,
  HLS_CONTENT_TYPE,
  LoadRequest,
  MediaInfo,
  castButtonVisible,
  loadCastSdk,
} from "./sender";

const URL_ = "https://node.example:8082/hls/aa/bb/index.m3u8";
const SWARM = "magnet:?xt=urn:ppspp:abc";

class FakeSession implements CastSession {
  loads: LoadRequest[] = [];
  refuse: string | null = null;
  media: { customData?: unknown } | null = null;

  getCastDevice() {
    return { friendlyName: "Living Room TV" };
  }

  loadMedia(request: LoadRequest): Promise<unknown> {
    this.loads.push(request);
    if (this.refuse) return Promise.reject(this.refuse);
    this.media = { customData: request.media.customData };
    return Promise.resolve(undefined);
  }

  getMediaSession() {
    return this.media ? { media: this.media } : null;
  }
}

class FakeContext implements CastContext {
  options: unknown = null;
  castState = "NOT_CONNECTED";
  session: FakeSession | null = null;
  /** What the picker does: start a session, or reject with an error code. */
  picker: "start" | string = "start";
  pickers = 0;
  ends: boolean[] = [];
  handlers = new Map<string, ((e: CastContextEvent) => void)[]>();

  setOptions(options: unknown) {
    this.options = options;
  }

  getCastState() {
    return this.castState;
  }

  getCurrentSession() {
    return this.session;
  }

  requestSession(): Promise<unknown> {
    this.pickers++;
    if (this.picker !== "start") return Promise.reject(this.picker);
    this.session = new FakeSession();
    this.emit("caststatechanged", { castState: "CONNECTED" });
    this.emit("sessionstatechanged", { sessionState: "SESSION_STARTED" });
    return Promise.resolve(undefined);
  }

  endCurrentSession(stopCasting: boolean) {
    this.ends.push(stopCasting);
    this.session = null;
    this.emit("sessionstatechanged", { sessionState: "SESSION_ENDED" });
  }

  addEventListener(type: string, handler: (e: CastContextEvent) => void) {
    this.handlers.set(type, [...(this.handlers.get(type) ?? []), handler]);
  }

  emit(type: string, e: CastContextEvent) {
    for (const h of this.handlers.get(type) ?? []) h(e);
  }
}

class FakeMediaInfo implements MediaInfo {
  constructor(public contentId: string, public contentType: string) {}
}

class FakeLoadRequest implements LoadRequest {
  autoplay: boolean | null = null;
  constructor(public media: MediaInfo) {}
}

const fakeSdk = (context: FakeContext): CastSdk => ({
  framework: {
    CastContext: { getInstance: () => context },
    CastContextEventType: {
      CAST_STATE_CHANGED: "caststatechanged",
      SESSION_STATE_CHANGED: "sessionstatechanged",
    },
    CastState: { NO_DEVICES_AVAILABLE: "NO_DEVICES_AVAILABLE", CONNECTING: "CONNECTING" },
    SessionState: {
      SESSION_STARTED: "SESSION_STARTED",
      SESSION_RESUMED: "SESSION_RESUMED",
      SESSION_ENDED: "SESSION_ENDED",
    },
  },
  cast: {
    AutoJoinPolicy: { ORIGIN_SCOPED: "origin_scoped" },
    media: {
      DEFAULT_MEDIA_RECEIVER_APP_ID: "CC1AD845",
      MediaInfo: FakeMediaInfo,
      LoadRequest: FakeLoadRequest,
      StreamType: { LIVE: "LIVE" },
      HlsSegmentFormat: { FMP4: "fmp4" },
      HlsVideoSegmentFormat: { FMP4: "fmp4" },
    },
  },
});

const sender = (context = new FakeContext()) => {
  const s = new CastSender(fakeSdk(context));
  const states: CastState[] = [];
  s.subscribe((state) => states.push(state));
  return { s, context, states };
};

describe("CastSender", () => {
  it("asks for the default media receiver, joining the origin's sessions", () => {
    const { context } = sender();
    assert.deepStrictEqual(context.options, {
      receiverApplicationId: "CC1AD845",
      autoJoinPolicy: "origin_scoped",
    });
  });

  it("starts a session from the picker and loads the HLS URL as live fMP4 HLS", async () => {
    const { s, context } = sender();
    await s.cast({ swarmUri: SWARM, url: Promise.resolve(URL_) });
    assert.strictEqual(context.pickers, 1);
    const [load] = context.session.loads;
    assert.deepStrictEqual(
      { ...load.media },
      {
        contentId: URL_,
        contentType: HLS_CONTENT_TYPE,
        contentUrl: URL_,
        streamType: "LIVE",
        hlsSegmentFormat: "fmp4",
        hlsVideoSegmentFormat: "fmp4",
        customData: { swarmUri: SWARM },
      }
    );
    assert.strictEqual(load.autoplay, true);
    assert.deepStrictEqual(
      { ...s.state() },
      {
        available: true,
        connecting: false,
        device: "Living Room TV",
        casting: SWARM,
        error: null,
      }
    );
  });

  it("loads into the session there is without the picker", async () => {
    const { s, context } = sender();
    context.session = new FakeSession();
    await s.cast({ swarmUri: SWARM, url: Promise.resolve(URL_) });
    assert.strictEqual(context.pickers, 0);
    assert.strictEqual(context.session.loads.length, 1);
    assert.strictEqual(s.state().casting, SWARM);
  });

  it("takes a closed picker as no failure, and says why other session failures happened", async () => {
    const { s, context } = sender();
    context.picker = "cancel";
    await s.cast({ swarmUri: SWARM, url: Promise.resolve(URL_) });
    assert.deepStrictEqual([s.state().error, s.state().connecting], [null, false]);
    context.picker = "receiver_unavailable";
    await s.cast({ swarmUri: SWARM, url: Promise.resolve(URL_) });
    assert.strictEqual(s.state().error, "casting failed: receiver_unavailable");
    assert.strictEqual(s.state().casting, null);
  });

  it("ends the session it started when the stream has no URL a receiver can reach", async () => {
    const { s, context } = sender();
    await s.cast({ swarmUri: SWARM, url: Promise.resolve<string | null>(null) });
    assert.strictEqual(s.state().error, "the node doesn't serve this stream as HLS");
    assert.deepStrictEqual(context.ends, [true]);
    await s.cast({
      swarmUri: SWARM,
      url: Promise.resolve("http://127.0.0.1:8082/hls/aa/bb/index.m3u8"),
    });
    assert.match(s.state().error, /can't reach the node's HLS address \(127\.0\.0\.1:8082\)/);
    assert.deepStrictEqual(context.ends, [true, true]);
    // A session it found stays.
    context.session = new FakeSession();
    await s.cast({ swarmUri: SWARM, url: Promise.resolve<string | null>(null) });
    assert.deepStrictEqual(context.ends, [true, true]);
  });

  it("says why the receiver refused the stream", async () => {
    const { s, context } = sender();
    context.session = new FakeSession();
    context.session.refuse = "session_error";
    await s.cast({ swarmUri: SWARM, url: Promise.resolve(URL_) });
    assert.strictEqual(s.state().error, "the receiver can't play the stream: session_error");
    assert.strictEqual(s.state().casting, null);
  });

  it("forgets the device and the stream when the session ends, which stopping does with the playback", async () => {
    const { s, context, states } = sender();
    await s.cast({ swarmUri: SWARM, url: Promise.resolve(URL_) });
    s.stop();
    assert.deepStrictEqual(context.ends, [true]);
    assert.deepStrictEqual([s.state().device, s.state().casting], [null, null]);
    assert.ok(states.length >= 3);
  });

  it("takes up a resumed session with the stream its receiver plays", () => {
    const context = new FakeContext();
    const session = new FakeSession();
    session.media = { customData: { swarmUri: SWARM } };
    const { s } = sender(context);
    context.session = session;
    context.emit("sessionstatechanged", { sessionState: "SESSION_RESUMED" });
    assert.deepStrictEqual([s.state().device, s.state().casting], ["Living Room TV", SWARM]);
  });

  it("is available while a receiver is on the network", () => {
    const { s, context } = sender();
    assert.ok(s.state().available);
    context.emit("caststatechanged", { castState: "NO_DEVICES_AVAILABLE" });
    assert.ok(!s.state().available);
    context.emit("caststatechanged", { castState: "CONNECTING" });
    assert.deepStrictEqual([s.state().available, s.state().connecting], [true, true]);
  });
});

describe("castButtonVisible", () => {
  it("shows with the SDK, a node that serves HLS and a receiver, and not without any", () => {
    const state = (available: boolean) => ({
      available,
      connecting: false,
      device: null,
      casting: null,
      error: null,
    });
    assert.ok(castButtonVisible(true, true, state(true)));
    assert.ok(!castButtonVisible(false, true, state(true)));
    assert.ok(!castButtonVisible(true, false, state(true)));
    assert.ok(!castButtonVisible(true, true, state(false)));
    assert.ok(!castButtonVisible(true, true, null));
  });
});

describe("loadCastSdk", () => {
  // A window whose document keeps the scripts appended to its head.
  const fakeWindow = (opts: { chrome: boolean; ua?: string; secure?: boolean }) => {
    const scripts: Record<string, unknown>[] = [];
    const timers: (() => void)[] = [];
    const win: Record<string, unknown> = {
      isSecureContext: opts.secure ?? true,
      navigator: { userAgent: opts.ua ?? "Mozilla/5.0 (Windows NT 10.0) Chrome/156.0.0.0" },
      document: {
        createElement: () => ({}),
        head: { appendChild: (s: Record<string, unknown>) => scripts.push(s) },
      },
      setTimeout: (f: () => void) => timers.push(f),
    };
    if (opts.chrome) win.chrome = {};
    return { win, scripts, timers };
  };

  type LoaderWindow = Parameters<typeof loadCastSdk>[0];

  it("loads nothing where the browser can't have Chrome's sender", async () => {
    for (const opts of [
      { chrome: false },
      { chrome: true, secure: false },
      { chrome: true, ua: "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7) CriOS/156.0" },
    ]) {
      const { win, scripts } = fakeWindow(opts);
      assert.strictEqual(await loadCastSdk(win as unknown as LoaderWindow), null);
      assert.deepStrictEqual(scripts, [], JSON.stringify(opts));
    }
  });

  it("loads the SDK once, and answers its API once it says the browser can cast", async () => {
    const { win, scripts } = fakeWindow({ chrome: true });
    const load = loadCastSdk(win as unknown as LoaderWindow);
    assert.strictEqual(loadCastSdk(win as unknown as LoaderWindow), load);
    assert.strictEqual(scripts.length, 1);
    assert.strictEqual(scripts[0].src, CAST_SDK_URL);
    const sdk = fakeSdk(new FakeContext());
    win.cast = { framework: sdk.framework };
    (win.chrome as Record<string, unknown>).cast = sdk.cast;
    (win.__onGCastApiAvailable as (ok: boolean) => void)(true);
    const loaded = await load;
    assert.strictEqual(loaded.framework, sdk.framework);
    assert.strictEqual(loaded.cast, sdk.cast);
  });

  it("answers null when the SDK says the browser can't cast, its script fails, or it says nothing", async () => {
    const refused = fakeWindow({ chrome: true });
    const a = loadCastSdk(refused.win as unknown as LoaderWindow);
    (refused.win.__onGCastApiAvailable as (ok: boolean) => void)(false);
    assert.strictEqual(await a, null);

    const failed = fakeWindow({ chrome: true });
    const b = loadCastSdk(failed.win as unknown as LoaderWindow);
    (failed.scripts[0].onerror as () => void)();
    assert.strictEqual(await b, null);

    const silent = fakeWindow({ chrome: true });
    const c = loadCastSdk(silent.win as unknown as LoaderWindow);
    for (const t of silent.timers) t();
    assert.strictEqual(await c, null);
  });
});
