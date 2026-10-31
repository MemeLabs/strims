// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  AudioDataLike,
  CoreEnv,
  DecoderLike,
  FrameLike,
  MAX_FRAMES_AHEAD,
  PlayerCore,
} from "./core";
import type {
  AudioOutput,
  DecoderTrack,
  DemuxedUnitOutput,
  DemuxerOutput,
  WorkletInput,
  WorkletReport,
} from "./protocol";

const RATE = 48000;
const QUANTUM = 128;
/** A step of the simulation: a quantum of the audio output. */
const STEP = QUANTUM / RATE;
const FPS = 30;
const AAC_US = (1024 * 1e6) / RATE;

const tracks = (audio: boolean): DecoderTrack[] => [
  {
    id: 1,
    kind: "video",
    codec: "avc1.64001f",
    description: new Uint8Array([1, 0x64, 0, 0x1f]),
    timescale: 90000,
    width: 640,
    height: 360,
  },
  ...(audio
    ? [
        {
          id: 2,
          kind: "audio" as const,
          codec: "mp4a.40.2",
          description: new Uint8Array([0x11, 0x90]),
          timescale: RATE,
          sampleRate: RATE,
          numberOfChannels: 2,
          preSkip: 0,
        },
      ]
    : []),
];

interface Arrival {
  at: number;
  outputs: DemuxerOutput[];
}

interface StreamOptions {
  audio?: boolean;
  /** Seconds of media. */
  seconds: number;
  /** Seconds between keyframes. */
  gop?: number;
  /** Seconds from a unit's presentation time to its arrival. */
  delay?: number;
  /** Seconds between arrivals: units come in batches, as signing makes them. */
  batch?: number;
  /** The viewer joins at this media time: the GOP before it comes at once. */
  join?: number;
  /** Video units decoding in [from, to) are lost: the next keyframe resumes video. */
  lose?: [number, number];
  /** No units arrive in [from, to) of arrival time; they come at its end. */
  outage?: [number, number];
}

const unit = (
  track: number,
  kind: "video" | "audio",
  pts: number,
  keyframe: boolean
): DemuxedUnitOutput => ({
  type: "unit",
  track,
  kind,
  keyframe,
  timestamp: Math.round(pts * 1e6),
  decodeTimestamp: Math.round(pts * 1e6),
  duration: kind === "audio" ? Math.round(AAC_US) : 0,
  published: 1_793_275_200_000 + pts * 1000,
  data: new Uint8Array(kind === "audio" ? 300 : 3000),
});

// A live stream's demuxer outputs, timed as a reader gets them.
const stream = (o: StreamOptions): Arrival[] => {
  const gop = o.gop ?? 2;
  const delay = o.delay ?? 0.3;
  const batch = o.batch ?? 0.25;
  const join = o.join ?? 0;
  const units: DemuxedUnitOutput[] = [];
  let a = 0;
  // The demuxer gives video after a loss from the next keyframe.
  const resume = o.lose ? Math.ceil(o.lose[0] / gop) * gop : Infinity;
  for (let k = 0; k < o.seconds * FPS; k++) {
    const pts = k / FPS;
    const key = k % Math.round(gop * FPS) === 0;
    if (!(o.lose && pts >= o.lose[0] && pts < resume - 1e-9)) {
      units.push(unit(1, "video", pts, key));
    }
    while (o.audio && (a * AAC_US) / 1e6 < (k + 1) / FPS) {
      units.push(unit(2, "audio", (a * AAC_US) / 1e6, true));
      a++;
    }
  }
  const gopStart = Math.floor(join / gop) * gop;
  const arrivals: Arrival[] = [
    { at: 0, outputs: [{ type: "configs", tracks: tracks(!!o.audio) }] },
  ];
  for (const u of units) {
    const pts = u.timestamp / 1e6;
    if (pts < gopStart) continue;
    let at =
      pts < join ? 0.05 + (pts - gopStart) * 0.05 : Math.ceil((pts + delay - join) / batch) * batch;
    if (o.outage && at >= o.outage[0] && at < o.outage[1]) at = o.outage[1];
    arrivals.push({ at, outputs: [u] });
  }
  if (o.lose) {
    const at = Math.ceil((o.lose[0] + delay - join) / batch) * batch;
    arrivals.push({ at, outputs: [{ type: "gap", track: 1, kind: "video", missing: 1 }] });
  }
  return arrivals.sort((x, y) => x.at - y.at);
};

class FakeVideo implements DecoderLike<VideoDecoderConfig> {
  state = "unconfigured";
  pending: number[] = [];
  constructor(
    private readonly world: World,
    private readonly output: (f: FrameLike) => void,
    private readonly error: (e: unknown) => void
  ) {}
  get decodeQueueSize() {
    return this.pending.length;
  }
  configure() {
    this.state = "configured";
  }
  decode(u: Omit<DemuxedUnitOutput, "type">) {
    if (this.state !== "configured") throw new Error(`decode while ${this.state}`);
    if (this.world.failAt !== null && u.timestamp >= this.world.failAt * 1e6) {
      this.world.failAt = null;
      this.world.later(() => {
        this.state = "closed";
        this.error(new Error("decoding error"));
      });
      return;
    }
    this.world.decodedKeys.push(u.keyframe);
    this.pending.push(u.timestamp);
    this.world.later(() => {
      if (this.state === "closed") return;
      const ts = this.pending.shift();
      this.world.held++;
      this.output({
        timestamp: ts,
        displayWidth: 640,
        displayHeight: 360,
        close: () => this.world.held--,
      });
    });
  }
  close() {
    this.state = "closed";
  }
}

class FakeAudio implements DecoderLike<AudioDecoderConfig> {
  state = "unconfigured";
  pending: number[] = [];
  constructor(private readonly world: World, private readonly output: (d: AudioDataLike) => void) {}
  get decodeQueueSize() {
    return this.pending.length;
  }
  configure() {
    this.state = "configured";
  }
  decode(u: Omit<DemuxedUnitOutput, "type">) {
    this.pending.push(u.timestamp);
    this.world.later(() => {
      if (this.state === "closed") return;
      this.output({
        timestamp: this.pending.shift(),
        numberOfFrames: 1024,
        sampleRate: RATE,
        close: () => undefined,
      });
    });
  }
  close() {
    this.state = "closed";
  }
}

// A simulated worker: the clock, decoders whose outputs come a step later,
// a worklet that plays a quantum a step, and what was shown.
class World implements CoreEnv {
  t = 0;
  private tickAt = Infinity;
  private tasks: (() => void)[] = [];
  rendered: { pts: number; at: number }[] = [];
  held = 0;
  maxHeld = 0;
  failAt: number | null = null;
  decodedKeys: boolean[] = [];
  videoDecoders = 0;
  failure: string | null = null;
  // The worklet.
  queue: number[] = [];
  consumed = 0;
  frame = 0;
  starved = 0;
  core: PlayerCore;

  constructor(output: AudioOutput | null) {
    this.core = new PlayerCore(this, output);
  }

  later(f: () => void) {
    this.tasks.push(f);
  }
  now() {
    return this.t;
  }
  wall() {
    return 1_793_275_200_000 + this.t * 1000;
  }
  videoDecoder(output: (f: FrameLike) => void, error: (e: unknown) => void) {
    this.videoDecoders++;
    return new FakeVideo(this, output, error);
  }
  audioDecoder(output: (d: AudioDataLike) => void) {
    return new FakeAudio(this, output);
  }
  supported() {
    return Promise.resolve(true);
  }
  planes(d: AudioDataLike) {
    return [new Float32Array(d.numberOfFrames), new Float32Array(d.numberOfFrames)];
  }
  render(f: FrameLike) {
    this.rendered.push({ pts: f.timestamp / 1e6, at: this.t });
    f.close();
  }
  worklet(m: WorkletInput) {
    if (m.type === "pcm") {
      this.queue.push(m.frames);
    } else {
      this.queue = [];
      this.consumed = m.index;
    }
  }
  schedule(delay: number) {
    this.tickAt = Math.min(this.tickAt, this.t + delay);
  }
  fail(message: string) {
    this.failure = message;
  }

  // Runs the simulation to `until`, delivering `arrivals` as they come.
  run(arrivals: Arrival[], until: number, running = true) {
    let i = 0;
    let quanta = 0;
    while (this.t < until) {
      while (i < arrivals.length && arrivals[i].at <= this.t) {
        this.core.outputs(arrivals[i].outputs, this.t);
        i++;
      }
      const tasks = this.tasks;
      this.tasks = [];
      for (const f of tasks) f();
      this.maxHeld = Math.max(this.maxHeld, this.held);
      if (running) this.playQuantum(quanta++);
      if (this.tickAt <= this.t) {
        this.tickAt = Infinity;
        this.core.tick();
      }
      this.t += STEP;
    }
    return arrivals.slice(i);
  }

  private playQuantum(q: number) {
    const at = this.consumed;
    const queued = this.queue.reduce((a, b) => a + b, 0);
    let need = QUANTUM;
    while (need > 0 && this.queue.length) {
      const take = Math.min(need, this.queue[0]);
      this.queue[0] -= take;
      need -= take;
      this.consumed += take;
      if (this.queue[0] === 0) this.queue.shift();
    }
    if (need > 0 && need < QUANTUM) this.starved++;
    if (q % 4 === 0) {
      const r: WorkletReport = {
        type: "report",
        frame: this.frame,
        consumed: at,
        queued,
        starved: this.starved,
      };
      this.core.report(r);
    }
    this.frame += QUANTUM;
  }
}

const AUDIO: AudioOutput = { sampleRate: RATE, running: true, latency: 0.02 };

// `arrivals` with an end output after the last.
const ended = (arrivals: Arrival[]): Arrival[] => [
  ...arrivals,
  { at: arrivals[arrivals.length - 1].at, outputs: [{ type: "end" }] },
];

// A later session's arrivals: `arrivals` from `at` on, its units presenting
// from `from` as the demuxer moves them, after a restart.
const later = (arrivals: Arrival[], at: number, from: number): Arrival[] => [
  { at, outputs: [{ type: "restart", offset: from }] },
  ...arrivals.map((a) => ({
    at: a.at + at,
    outputs: a.outputs.map((o) =>
      o.type === "unit"
        ? {
            ...o,
            timestamp: o.timestamp + from * 1e6,
            decodeTimestamp: o.decodeTimestamp + from * 1e6,
            published: o.published + at * 1000,
          }
        : o
    ),
  })),
];

describe("PlayerCore", () => {
  it("joins at its target lag, decoding from the keyframe without showing what comes before", () => {
    const w = new World(AUDIO);
    const arrivals = stream({ audio: true, seconds: 20, join: 5.5 });
    w.run(arrivals, 8);
    const s = w.core.stats();
    assert.ok(s.joined, "not joined");
    assert.strictEqual(s.clock, "audio");
    // The first picture shown is at the start, past the keyframe at 4 s.
    const first = w.rendered[0];
    assert.ok(first.pts > 4.5, `${first.pts}`);
    assert.ok(s.framesSkipped > 10, `${s.framesSkipped}`);
    assert.ok(w.decodedKeys[0], "decoding started mid-GOP");
    // Held at the target, give or take a frame and the batches.
    assert.ok(Math.abs(s.lag - s.target) < 0.2, `lag ${s.lag} target ${s.target}`);
    assert.strictEqual(s.stalls, 0);
    // Pictures show in order, at the audio's pace.
    for (let i = 1; i < w.rendered.length; i++) {
      assert.ok(w.rendered[i].pts > w.rendered[i - 1].pts);
    }
    assert.ok(s.avOffset.p95 < 20, `${s.avOffset.p95}`);
    assert.ok(w.maxHeld <= MAX_FRAMES_AHEAD, `${w.maxHeld}`);
  });

  it("holds the last picture over a lost one until the next keyframe, while audio plays on", () => {
    const w = new World(AUDIO);
    w.run(stream({ audio: true, seconds: 20, lose: [7.0, 7.5] }), 12);
    const s = w.core.stats();
    assert.ok(s.joined);
    assert.strictEqual(s.gaps, 1);
    const shown = w.rendered.map((r) => r.pts);
    assert.ok(
      !shown.some((p) => p >= 7.0 && p < 8.0),
      "pictures shown between the loss and the keyframe"
    );
    assert.ok(shown.some((p) => p >= 8.0));
    // The picture before the loss held while the clock ran on.
    const before = w.rendered.filter((r) => r.pts < 7).pop();
    const after = w.rendered.find((r) => r.pts >= 8);
    assert.ok(after.at - before.at > 0.9, `${after.at - before.at}`);
    assert.strictEqual(s.stalls, 0);
  });

  it("a decoder error rebuilds the decoder, which resumes at a keyframe", () => {
    const w = new World(AUDIO);
    w.failAt = 7.2;
    w.run(stream({ audio: true, seconds: 20 }), 12);
    const s = w.core.stats();
    assert.strictEqual(s.decoderErrors, 1);
    assert.strictEqual(w.videoDecoders, 2);
    assert.ok(!w.rendered.some((r) => r.pts > 7.25 && r.pts < 8), "pictures before the keyframe");
    assert.ok(w.rendered.some((r) => r.pts >= 8));
    assert.strictEqual(w.failure, null);
  });

  it("an outage stalls the clock, raises the target, and play resumes", () => {
    const w = new World(AUDIO);
    const arrivals = stream({ audio: true, seconds: 30, outage: [8, 9] });
    const rest = w.run(arrivals, 8);
    const target = w.core.stats().target;
    w.run(rest, 16);
    const s = w.core.stats();
    assert.strictEqual(s.stalls, 1);
    // The clock held from where the media ran out to the outage's end.
    assert.ok(s.stallTime > 0.3 && s.stallTime < 1, `${s.stallTime}`);
    assert.ok(s.target > target, `${s.target} after ${target}`);
    // Pictures go on after it.
    assert.ok(w.rendered[w.rendered.length - 1].pts > 14);
  });

  it("video alone plays on the worker's clock", () => {
    const w = new World(null);
    w.run(stream({ seconds: 15 }), 10, false);
    const s = w.core.stats();
    assert.ok(s.joined);
    assert.strictEqual(s.clock, "free");
    assert.ok(Math.abs(s.lag - s.target) < 0.2, `lag ${s.lag} target ${s.target}`);
    assert.ok(w.rendered.length > 200);
  });

  it("plays silently on its own clock until the audio output runs, then on the output's", () => {
    const w = new World({ ...AUDIO, running: false });
    const arrivals = stream({ audio: true, seconds: 20 });
    const rest = w.run(arrivals, 6, false);
    let s = w.core.stats();
    assert.ok(s.joined);
    assert.ok(s.silent);
    assert.strictEqual(s.clock, "free");
    w.core.setAudio(AUDIO);
    w.run(rest, 10);
    s = w.core.stats();
    assert.ok(!s.silent);
    assert.strictEqual(s.clock, "audio");
    assert.ok(Math.abs(s.lag - s.target) < 0.3, `lag ${s.lag} target ${s.target}`);
  });

  it("skips ahead to its target when far behind", () => {
    const w = new World(AUDIO);
    const arrivals = stream({ audio: true, seconds: 30 });
    // A viewer that paused, then plays: it skips to the target.
    w.run(arrivals, 8);
    w.core.setPaused(true);
    const rest = w.run(
      arrivals.filter((a) => a.at > 8),
      11
    );
    w.core.setPaused(false);
    w.run(rest, 16);
    const s = w.core.stats();
    assert.ok(Math.abs(s.lag - s.target) < 0.2, `lag ${s.lag} target ${s.target}`);
    // Pictures from about the target lag behind the edge, 1 s behind now.
    assert.ok(w.rendered[w.rendered.length - 1].pts > 14.5);
  });

  it("plays out to the end of an ended stream and holds its last picture, with no stall", () => {
    const w = new World(AUDIO);
    w.run(ended(stream({ audio: true, seconds: 10 })), 16);
    const s = w.core.stats();
    assert.ok(s.ended);
    assert.strictEqual(s.stalls, 0);
    assert.strictEqual(s.skips, 0);
    assert.strictEqual(s.stallTime, 0);
    assert.strictEqual(s.audioUnderruns, 0);
    // The last picture showed, and holds.
    const last = w.rendered[w.rendered.length - 1];
    assert.ok(Math.abs(last.pts - 299 / FPS) < 1e-6, `${last.pts}`);
    assert.ok(last.at < 11.5, `${last.at}`);
    const c = w.core.clock();
    assert.strictEqual(c.rate, 0);
    assert.strictEqual(c.lag, null);
    assert.ok(c.published !== null);
  });

  it("video alone plays out to the end on the worker's clock", () => {
    const w = new World(null);
    w.run(ended(stream({ seconds: 10 })), 16, false);
    const s = w.core.stats();
    assert.ok(s.ended);
    assert.strictEqual(s.stalls, 0);
    assert.ok(Math.abs(w.rendered[w.rendered.length - 1].pts - 299 / FPS) < 1e-6);
  });

  it("a later session's media after the end joins again and plays on", () => {
    const w = new World(AUDIO);
    const first = ended(stream({ audio: true, seconds: 10 }));
    const second = later(stream({ audio: true, seconds: 10 }), 20, 10);
    const rest = w.run(
      [...first, ...second].sort((x, y) => x.at - y.at),
      18
    );
    assert.ok(w.core.stats().ended);
    const shown = w.rendered.length;
    w.run(rest, 28);
    const s = w.core.stats();
    assert.ok(!s.ended);
    assert.strictEqual(s.stalls, 0);
    assert.ok(w.rendered.length > shown + 100, `${w.rendered.length - shown}`);
    assert.ok(
      w.rendered[w.rendered.length - 1].pts > 16,
      `${w.rendered[w.rendered.length - 1].pts}`
    );
    assert.ok(Math.abs(s.lag - s.target) < 0.3, `lag ${s.lag} target ${s.target}`);
  });

  it("an end during a stall plays out there, ending the stall", () => {
    const w = new World(AUDIO);
    const arrivals = stream({ audio: true, seconds: 10 });
    // The media up to 6 s comes; then the stream ends.
    const cut = arrivals.filter(
      (a) => !a.outputs.some((o) => o.type === "unit" && o.timestamp >= 6e6)
    );
    w.run([...cut, { at: 9, outputs: [{ type: "end" }] }], 12);
    const s = w.core.stats();
    assert.ok(s.ended);
    assert.strictEqual(s.stalls, 1);
    // From where the media ran out, about 7 s, to the end's arrival.
    assert.ok(s.stallTime > 1.5 && s.stallTime < 2.5, `${s.stallTime}`);
    w.run([], 14);
    assert.strictEqual(w.core.stats().stallTime, s.stallTime);
  });

  it("publish times follow the presented media", () => {
    const w = new World(AUDIO);
    w.run(stream({ audio: true, seconds: 15 }), 10);
    const c = w.core.clock();
    // Units were published at their media times: the lag behind publishing
    // is the stream's delay plus the playout's lag behind the edge, about.
    assert.ok(c.published !== null);
    const lag = (c.at - c.published) / 1000;
    assert.ok(lag > c.targetLag && lag < c.targetLag + 0.6, `${lag} vs ${c.targetLag}`);
    assert.ok(c.rate > 0.97 && c.rate < 1.03);
  });
});
