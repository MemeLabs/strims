// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { PlayerKind } from "./players";
import {
  Assessment,
  DecodingInfo,
  FAILURES_STORAGE_KEY,
  FAILURE_TTL_MS,
  MAX_CANDIDATES,
  MAX_FAILURES,
  Probes,
  Rendition,
  RenditionFailures,
  SnippetRenditions,
  assessRenditions,
  browserProbes,
  mseType,
  rankRenditions,
  renditionCandidates,
  swarmId,
} from "./renditions";

const uri = (id: string, options = "&x.cs=4096") => `magnet:?xt=urn:ppspp:${id}${options}`;
const units = (...codecs: string[]) => `video/x.strims-units;codecs="${codecs.join(",")}"`;

const MAIN = { swarmUri: uri("main"), mimeType: units("avc1.640028", "mp4a.40.2") };

const listed = (
  id: string,
  video: string,
  width: number,
  height: number,
  frameRate = 30,
  bitrate = 3_000_000
) => ({
  swarmUri: uri(id),
  mimeType: units(video, "mp4a.40.2"),
  bitrate: BigInt(bitrate),
  width,
  height,
  frameRate,
});

const snippet = (...renditions: SnippetRenditions["renditions"]): SnippetRenditions => ({
  renditions,
  videoWidth: 1920,
  videoHeight: 1080,
});

const rendition = (
  id: string,
  video: string,
  height: number,
  extra: Partial<Rendition> = {}
): Rendition => ({
  swarmUri: uri(id),
  id: `urn:ppspp:${id}`,
  mimeType: units(video, "mp4a.40.2"),
  video,
  audio: ["mp4a.40.2"],
  width: Math.round((height * 16) / 9),
  height,
  frameRate: 30,
  bitrate: 3_000_000,
  main: false,
  ...extra,
});

const assess = (r: Rendition, powerEfficient = true, supported = true): Assessment => ({
  rendition: r,
  supported,
  powerEfficient,
});

const ids = (rs: Rendition[]) => rs.map((r) => r.id.replace("urn:ppspp:", ""));

const desktop = (height: number) => ({
  width: Math.round((height * 16) / 9),
  height,
  portable: false,
});
const phone = (height: number) => ({
  width: Math.round((height * 16) / 9),
  height,
  portable: true,
});

describe("renditionCandidates", () => {
  it("is the listing's own alone without a snippet or renditions", () => {
    for (const s of [null, snippet()]) {
      const [only, ...rest] = renditionCandidates(MAIN, s);
      assert.deepStrictEqual(rest, []);
      assert.strictEqual(only.main, true);
      assert.strictEqual(only.swarmUri, MAIN.swarmUri);
      assert.strictEqual(only.video, "avc1.640028");
    }
    assert.strictEqual(renditionCandidates(MAIN, snippet())[0].height, 1080);
  });

  it("finds the listing's own by its swarm id, whatever options its URI names", () => {
    const c = renditionCandidates(
      MAIN,
      snippet(
        { ...listed("main", "avc1.640028", 1920, 1080, 60, 6_000_000), swarmUri: uri("main", "") },
        listed("source", "hvc1.1.6.L123.B0", 1920, 1080, 60, 8_000_000)
      )
    );
    assert.deepStrictEqual(ids(c), ["main", "source"]);
    assert.strictEqual(c[0].main, true);
    assert.strictEqual(c[0].swarmUri, MAIN.swarmUri);
    assert.strictEqual(c[0].mimeType, MAIN.mimeType);
    assert.strictEqual(c[0].frameRate, 60);
    assert.strictEqual(c[0].bitrate, 6_000_000);
    assert.deepStrictEqual(
      [c[1].video, c[1].audio, c[1].width, c[1].height],
      ["hvc1.1.6.L123.B0", ["mp4a.40.2"], 1920, 1080]
    );
  });

  it("adds the listing's own when the snippet doesn't name it", () => {
    const c = renditionCandidates(MAIN, snippet(listed("480p30", "avc1.64001e", 854, 480)));
    assert.deepStrictEqual(ids(c), ["480p30", "main"]);
    assert.deepStrictEqual([c[1].main, c[1].width, c[1].height], [true, 1920, 1080]);
  });

  it("takes each swarm once, and skips renditions it can't rank", () => {
    const c = renditionCandidates(
      MAIN,
      snippet(
        listed("a", "av01.0.08M.08", 1920, 1080),
        listed("a", "hvc1.1.6.L123.B0", 1920, 1080),
        {
          ...listed("other", "avc1.64001f", 1280, 720),
          mimeType: 'video/mp4;codecs="avc1.64001f"',
        },
        { ...listed("nocodecs", "avc1.64001f", 1280, 720), mimeType: "video/x.strims-units" },
        { ...listed("vp9", "avc1.64001f", 1280, 720), mimeType: units("vp09.00.10.08") },
        {
          ...listed("two", "avc1.64001f", 1280, 720),
          mimeType: units("avc1.64001f", "hvc1.1.6.L93.B0"),
        },
        { ...listed("long", "avc1.64001f", 1280, 720), mimeType: units(`avc1.${"0".repeat(80)}`) },
        { ...listed("odd", "avc1.64001f", 1280, 720), mimeType: units("avc1.64001f;x") },
        listed("nosize", "avc1.64001f", 0, 0),
        listed("huge", "avc1.64001f", 1 << 20, 720),
        { ...listed("nouri", "avc1.64001f", 1280, 720), swarmUri: "" },
        { ...listed("audio", "avc1.64001f", 1280, 720), mimeType: units("mp4a.40.2") }
      )
    );
    assert.deepStrictEqual(ids(c), ["a", "main"]);
    assert.strictEqual(c[0].video, "av01.0.08M.08");
  });

  it("takes unknown frame rates and bitrates as unknown", () => {
    const [r] = renditionCandidates(
      MAIN,
      snippet({
        ...listed("r", "avc1.64001f", 1280, 720, NaN),
        bitrate: BigInt(Number.MAX_SAFE_INTEGER) * BigInt(4),
      })
    );
    assert.deepStrictEqual([r.frameRate, r.bitrate], [0, 0]);
  });

  it("holds at most MAX_CANDIDATES, the listing's own among them", () => {
    const many = Array.from({ length: 40 }, (_, i) => listed(`r${i}`, "avc1.64001e", 854, 480));
    const c = renditionCandidates(MAIN, snippet(...many));
    assert.strictEqual(c.length, MAX_CANDIDATES);
    assert.strictEqual(c.filter((r) => r.main).length, 1);
  });

  it("names a swarm by its magnet URI's xt", () => {
    assert.strictEqual(swarmId(uri("abc")), "urn:ppspp:abc");
    assert.strictEqual(swarmId("not a uri"), "not a uri");
  });
});

describe("mseType", () => {
  it("is the remuxer's init type: video codec first, Opus as MP4 names it", () => {
    assert.strictEqual(
      mseType(rendition("a", "hvc1.1.6.L123.B0", 1080)),
      'video/mp4;codecs="hvc1.1.6.L123.B0,mp4a.40.2"'
    );
    assert.strictEqual(
      mseType(rendition("a", "av01.0.08M.08", 1080, { audio: ["opus"] })),
      'video/mp4;codecs="av01.0.08M.08,Opus"'
    );
  });
});

// Probes answering from tables, recording what decodingInfo was asked.
const probes = (
  mse: Record<string, boolean>,
  info: Record<string, DecodingInfo | null>,
  webCodecs?: Record<string, boolean>
) => {
  const asked: VideoConfiguration[] = [];
  const p: Probes = {
    mse: (type) => Object.entries(mse).some(([codec, ok]) => ok && type.includes(codec)),
    decodingInfo: (video) => {
      asked.push(video);
      const codec = /codecs="([^"]*)"/.exec(video.contentType)[1];
      return Promise.resolve(info[codec] ?? null);
    },
    webCodecs: webCodecs && ((codec) => Promise.resolve(webCodecs[codec] ?? false)),
  };
  return { p, asked };
};

describe("assessRenditions", () => {
  const main = rendition("main", "avc1.640028", 1080, { main: true });
  const hevc = rendition("hevc", "hvc1.1.6.L123.B0", 1080);
  const av1 = rendition("av1", "av01.0.08M.08", 1080);
  const ok = { supported: true, powerEfficient: true };

  it("gates on the sink's type and decodingInfo, and asks of the video codec alone", async () => {
    const { p, asked } = probes(
      { avc1: true, hvc1: true, av01: true },
      {
        "avc1.640028": ok,
        "hvc1.1.6.L123.B0": ok,
        "av01.0.08M.08": { supported: false, powerEfficient: false },
      }
    );
    const a = await assessRenditions([main, hevc, av1], p, () => false);
    assert.deepStrictEqual(
      a.map((x) => [x.rendition.id, x.supported, x.powerEfficient]),
      [
        ["urn:ppspp:main", true, true],
        ["urn:ppspp:hevc", true, true],
        ["urn:ppspp:av1", false, false],
      ]
    );
    assert.ok(
      asked.every((v) => !v.contentType.includes("mp4a") && v.framerate > 0 && v.bitrate > 0)
    );
    const none = await assessRenditions(
      [main, hevc],
      probes({ avc1: true }, { "hvc1.1.6.L123.B0": ok }).p,
      () => false
    );
    assert.deepStrictEqual(
      none.map((x) => x.supported),
      [true, false]
    );
  });

  it("reads no decodingInfo as supported, but not power-efficient", async () => {
    const a = await assessRenditions([main, hevc], probes({ hvc1: true }, {}).p, () => false);
    assert.deepStrictEqual(
      a.map((x) => [x.supported, x.powerEfficient]),
      [
        [true, false],
        [true, false],
      ]
    );
  });

  it("needs WebCodecs too for the low-latency player", async () => {
    const both = probes({ hvc1: true, av01: true }, {}, { "hvc1.1.6.L123.B0": true });
    const a = await assessRenditions([hevc, av1], both.p, () => false);
    assert.deepStrictEqual(
      a.map((x) => x.supported),
      [true, false]
    );
  });

  it("skips renditions that failed here, but never the listing's own", async () => {
    const { p } = probes({ avc1: false, hvc1: true }, {});
    const a = await assessRenditions([main, hevc], p, () => true);
    assert.deepStrictEqual(
      a.map((x) => x.supported),
      [true, false]
    );
  });
});

describe("browserProbes", () => {
  it("asks ManagedMediaSource first, WebCodecs only for the low-latency player, and answers no when a probe throws", async () => {
    const calls: string[] = [];
    const scope = {
      ManagedMediaSource: { isTypeSupported: (t: string) => (calls.push(`mms ${t}`), true) },
      MediaSource: { isTypeSupported: () => false },
      VideoDecoder: {
        isConfigSupported: () => Promise.reject(new TypeError("no")),
      },
      navigator: {
        mediaCapabilities: {
          decodingInfo: (c: MediaDecodingConfiguration) => {
            calls.push(`info ${c.type} ${c.video.contentType}`);
            return Promise.resolve({
              supported: true,
              smooth: false,
              powerEfficient: true,
              keySystemAccess: null,
            });
          },
        },
      },
    };
    const baseline = browserProbes(PlayerKind.Units, scope);
    assert.strictEqual(baseline.mse("video/mp4"), true);
    assert.strictEqual(baseline.webCodecs, undefined);
    const video = {
      contentType: 'video/mp4;codecs="avc1.64001f"',
      width: 1,
      height: 1,
      bitrate: 1,
      framerate: 1,
    };
    assert.deepStrictEqual(await baseline.decodingInfo(video), {
      supported: true,
      powerEfficient: true,
    });
    await baseline.decodingInfo(video);
    assert.deepStrictEqual(calls, [
      "mms video/mp4",
      'info media-source video/mp4;codecs="avc1.64001f"',
    ]);
    const ll = browserProbes(PlayerKind.LowLatency, scope);
    assert.strictEqual(await ll.webCodecs("avc1.64001f", 1, 1), false);

    const throwing = browserProbes(PlayerKind.Units, {
      MediaSource: {
        isTypeSupported: () => {
          throw new Error("no");
        },
      },
      navigator: {
        mediaCapabilities: {
          decodingInfo: () => Promise.reject(new TypeError("no")),
        },
      },
    });
    assert.strictEqual(throwing.mse("video/mp4"), false);
    assert.strictEqual(await throwing.decodingInfo(video), null);
    assert.strictEqual(await browserProbes(PlayerKind.Units, {}).decodingInfo(video), null);
  });
});

describe("rankRenditions", () => {
  const main = rendition("main", "avc1.640028", 1080, { main: true, bitrate: 6_000_000 });
  const r720 = rendition("720p", "avc1.640020", 720);
  const r480 = rendition("480p", "avc1.64001e", 480, { bitrate: 1_200_000 });

  it("plays the smallest rendition that covers the player, and none after the listing's own", () => {
    const all = [assess(main), assess(r720), assess(r480)];
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(1080))), ["main"]);
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(700))), ["720p", "main"]);
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(300))), ["480p", "720p", "main"]);
  });

  it("takes a rendition enlarged by at most FIT_SLACK as covering", () => {
    const all = [assess(main), assess(r720), assess(r480)];
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(750))), ["720p", "main"]);
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(770))), ["main"]);
  });

  it("plays the largest rendition when none covers the player", () => {
    const all = [assess(r480), assess(r720), assess(main)];
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(2160))), ["main"]);
    const small = rendition("small", "avc1.64001e", 480, { main: true });
    assert.deepStrictEqual(ids(rankRenditions([assess(small), assess(r720)], desktop(2160))), [
      "720p",
      "small",
    ]);
  });

  it("puts power-efficient decoding before fit", () => {
    const av1 = rendition("av1", "av01.0.04M.08", 480);
    const all = [assess(main), assess(av1, false), assess(r720)];
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(300))), ["720p", "main"]);
  });

  it("caps a portable device's picture at PORTABLE_MAX_SIDE unless nothing else is there", () => {
    const hevc = rendition("source", "hvc1.1.6.L123.B0", 1080);
    const all = [assess(main), assess(hevc), assess(r720), assess(r480)];
    assert.deepStrictEqual(ids(rankRenditions(all, phone(1125))), [
      "720p",
      "480p",
      "source",
      "main",
    ]);
    assert.deepStrictEqual(ids(rankRenditions(all, phone(633))), [
      "720p",
      "480p",
      "source",
      "main",
    ]);
    assert.deepStrictEqual(ids(rankRenditions([assess(main), assess(hevc)], phone(633))), [
      "source",
      "main",
    ]);
  });

  it("prefers, at one size, a higher frame rate, then the newer codec, when both decode power-efficiently", () => {
    const hevc = rendition("hevc", "hvc1.1.6.L123.B0", 1080, { bitrate: 4_000_000 });
    const av1 = rendition("av1", "av01.0.08M.08", 1080, { bitrate: 3_000_000 });
    const h60 = rendition("h60", "avc1.64002a", 1080, { frameRate: 60 });
    assert.deepStrictEqual(
      ids(rankRenditions([assess(main), assess(hevc), assess(av1)], desktop(1080))),
      ["av1", "hevc", "main"]
    );
    assert.deepStrictEqual(
      ids(rankRenditions([assess(main), assess(hevc), assess(h60)], desktop(1080))),
      ["h60", "hevc", "main"]
    );
  });

  it("prefers, at one size, the codec cheapest to decode when none decodes power-efficiently", () => {
    const hevc = rendition("hevc", "hvc1.1.6.L123.B0", 1080);
    const av1 = rendition("av1", "av01.0.08M.08", 1080, { frameRate: 60, bitrate: 9_000_000 });
    const all = [assess(av1, false), assess(hevc, false), assess(main, false)];
    assert.deepStrictEqual(ids(rankRenditions(all, desktop(1080))), ["main"]);
    const noMainYet = [
      assess(av1, false),
      assess(hevc, false),
      assess(r720, false),
      assess(main, false),
    ];
    assert.deepStrictEqual(ids(rankRenditions(noMainYet, desktop(700))), ["720p", "main"]);
    const without = [
      assess(av1, false),
      assess(hevc, false),
      assess({ ...main, video: "avc1.640028" }, true),
    ];
    assert.deepStrictEqual(ids(rankRenditions(without, desktop(1080))), ["main"]);
  });

  it("puts the listing's own after a rendition it ties with, and skips unsupported ones", () => {
    const twin = rendition("twin", "avc1.640028", 1080, { bitrate: 6_000_000 });
    const gone = rendition("gone", "av01.0.08M.08", 1080);
    assert.deepStrictEqual(
      ids(rankRenditions([assess(main), assess(gone, true, false), assess(twin)], desktop(1080))),
      ["twin", "main"]
    );
  });

  it("takes a listing's own of unknown size as large as the largest", () => {
    const unsized = { ...main, width: 0, height: 0 };
    assert.deepStrictEqual(ids(rankRenditions([assess(unsized), assess(r720)], desktop(1080))), [
      "main",
    ]);
    assert.deepStrictEqual(ids(rankRenditions([assess(unsized)], desktop(1080))), ["main"]);
  });

  it("plays the largest within the cap when the player's size is unknown", () => {
    const all = [assess(main), assess(r720), assess(r480)];
    assert.deepStrictEqual(ids(rankRenditions(all, { width: 0, height: 0, portable: false })), [
      "main",
    ]);
    assert.deepStrictEqual(ids(rankRenditions(all, { width: 0, height: 0, portable: true })), [
      "720p",
      "480p",
      "main",
    ]);
  });
});

// A local storage of one map, which can be made to throw.
const memory = () => {
  const items = new Map<string, string>();
  let throws = false;
  return {
    items,
    throw: () => (throws = true),
    getItem: (k: string) => {
      if (throws) throw new Error("denied");
      return items.get(k) ?? null;
    },
    setItem: (k: string, v: string) => {
      if (throws) throw new Error("denied");
      items.set(k, v);
    },
  };
};

describe("RenditionFailures", () => {
  const hevc = rendition("hevc", "hvc1.2.4.L153.B0", 1080);
  const av1 = rendition("av1", "av01.0.08M.08", 1080);
  const h264 = rendition("720p", "avc1.640020", 720);

  it("skips a codec that failed in a player, for that player, until its TTL", () => {
    const storage = memory();
    let now = 1_000_000;
    const f = new RenditionFailures(storage, "ua", () => now);
    f.record(PlayerKind.Units, hevc);
    assert.strictEqual(f.failed(PlayerKind.Units, hevc), true);
    assert.strictEqual(f.failed(PlayerKind.LowLatency, hevc), false);
    assert.strictEqual(f.failed(PlayerKind.Units, av1), false);
    const again = new RenditionFailures(storage, "ua", () => now);
    assert.strictEqual(again.failed(PlayerKind.Units, hevc), true);
    now += FAILURE_TTL_MS;
    assert.strictEqual(again.failed(PlayerKind.Units, hevc), false);
    assert.strictEqual(
      new RenditionFailures(storage, "ua", () => now).failed(PlayerKind.Units, hevc),
      false
    );
  });

  it("keeps no H.264 failure", () => {
    const storage = memory();
    const f = new RenditionFailures(storage, "ua");
    f.record(PlayerKind.Units, h264);
    assert.strictEqual(f.failed(PlayerKind.Units, h264), false);
    assert.strictEqual(storage.items.size, 0);
  });

  it("starts over for another browser", () => {
    const storage = memory();
    new RenditionFailures(storage, "ua 1").record(PlayerKind.Units, hevc);
    assert.strictEqual(
      new RenditionFailures(storage, "ua 2").failed(PlayerKind.Units, hevc),
      false
    );
  });

  it("keeps at most MAX_FAILURES, the newest", () => {
    const storage = memory();
    let now = 0;
    const f = new RenditionFailures(storage, "ua", () => now);
    for (let i = 0; i <= MAX_FAILURES; i++) {
      now++;
      f.record(PlayerKind.Units, rendition(`r${i}`, `hvc1.1.6.L${i}.B0`, 1080));
    }
    const stored = JSON.parse(storage.items.get(FAILURES_STORAGE_KEY)) as {
      entries: Record<string, number>;
    };
    assert.strictEqual(Object.keys(stored.entries).length, MAX_FAILURES);
    assert.strictEqual(f.failed(PlayerKind.Units, rendition("r0", "hvc1.1.6.L0.B0", 1080)), false);
    assert.strictEqual(f.failed(PlayerKind.Units, rendition("r1", "hvc1.1.6.L1.B0", 1080)), true);
  });

  it("holds none where storage throws or holds something else", () => {
    const storage = memory();
    storage.items.set(FAILURES_STORAGE_KEY, "{not json");
    assert.strictEqual(new RenditionFailures(storage, "ua").failed(PlayerKind.Units, hevc), false);
    storage.items.set(
      FAILURES_STORAGE_KEY,
      JSON.stringify({ ua: "ua", entries: { [`units|${hevc.video}`]: "x" } })
    );
    assert.strictEqual(new RenditionFailures(storage, "ua").failed(PlayerKind.Units, hevc), false);
    storage.throw();
    const f = new RenditionFailures(storage, "ua");
    f.record(PlayerKind.Units, hevc);
    assert.strictEqual(f.failed(PlayerKind.Units, hevc), true);
    const none = new RenditionFailures(null, "ua");
    none.record(PlayerKind.Units, hevc);
    assert.strictEqual(none.failed(PlayerKind.Units, hevc), true);
  });
});
