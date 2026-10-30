// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The low-latency player's audio output: an AudioWorklet that plays the
// samples the player worker sends it over a port of their own, through a
// gain that follows the video's volume.

import type { AudioOutput } from "./protocol";

/** The rate of the page's shared context: Opus's, and most streams' AAC. */
export const CONTEXT_RATE = 48_000;
/** Milliseconds between looks at the output's latency, which moves with the device. */
export const LATENCY_POLL_MS = 1000;
/** The processor's name. */
export const WORKLET_NAME = "strims-playout";

/**
 * The worklet: a queue of planar blocks played in order, silence when it
 * runs out. It reports every 4 quanta, and when it runs out or resumes,
 * the sample it played at a quantum's first context frame. A reset drops
 * the queue and renumbers the samples that follow.
 */
export const WORKLET_SOURCE = `
registerProcessor("${WORKLET_NAME}", class extends AudioWorkletProcessor {
  constructor() {
    super();
    this.queue = [];
    this.head = 0;
    this.queued = 0;
    this.consumed = 0;
    this.starved = 0;
    this.short = true;
    this.quanta = 0;
    this.feed = null;
    this.stopped = false;
    this.port.onmessage = ({ data }) => {
      if (data === "stop") {
        this.stopped = true;
      } else if (data && data.port) {
        this.feed = data.port;
        this.feed.onmessage = (m) => this.input(m.data);
      }
    };
  }
  input(m) {
    if (m.type === "pcm") {
      this.queue.push(m);
      this.queued += m.frames;
    } else if (m.type === "reset") {
      this.queue = [];
      this.head = 0;
      this.queued = 0;
      this.consumed = m.index;
    }
  }
  process(inputs, outputs) {
    if (this.stopped) return false;
    const out = outputs[0];
    const n = out[0] ? out[0].length : 128;
    const at = this.consumed;
    const queued = this.queued;
    let i = 0;
    while (i < n && this.queue.length) {
      const b = this.queue[0];
      const take = Math.min(n - i, b.frames - this.head);
      for (let c = 0; c < out.length; c++) {
        const src = b.channels[Math.min(c, b.channels.length - 1)];
        out[c].set(src.subarray(this.head, this.head + take), i);
      }
      i += take;
      this.head += take;
      this.consumed += take;
      this.queued -= take;
      if (this.head >= b.frames) {
        this.queue.shift();
        this.head = 0;
      }
    }
    const short = i < n;
    if (short && !this.short) this.starved++;
    if (++this.quanta % 4 === 0 || short !== this.short) {
      if (this.feed) {
        this.feed.postMessage({ type: "report", frame: currentFrame, consumed: at, queued, starved: this.starved });
      }
    }
    this.short = short;
    return true;
  }
});
`;

const modules = new WeakMap<BaseAudioContext, Promise<void>>();

// Adds the worklet's module to a context, once.
const loadModule = (ctx: BaseAudioContext): Promise<void> => {
  let p = modules.get(ctx);
  if (!p) {
    const url = URL.createObjectURL(new Blob([WORKLET_SOURCE], { type: "text/javascript" }));
    p = ctx.audioWorklet.addModule(url).finally(() => URL.revokeObjectURL(url));
    modules.set(ctx, p);
  }
  return p;
};

let shared: AudioContext | null = null;

// The page's AudioContext for players, at CONTEXT_RATE (the browser
// resamples to the device's), made on first use. Browsers start it only
// after a gesture: it resumes at the page's next tap or key press, and
// until then players play silently.
export const sharedAudioContext = (): AudioContext | null => {
  if (shared) return shared;
  if (typeof AudioContext !== "function") return null;
  try {
    shared = new AudioContext({ sampleRate: CONTEXT_RATE, latencyHint: "interactive" });
  } catch {
    return null;
  }
  const ctx = shared;
  const resume = () => {
    if (ctx.state !== "running") void ctx.resume().catch(() => undefined);
  };
  for (const type of ["pointerdown", "pointerup", "keydown", "touchend"]) {
    document.addEventListener(type, resume, { capture: true, passive: true });
  }
  resume();
  return ctx;
};

/**
 * A player's audio output on a context: the worklet node, a gain that
 * follows the video's muting and volume, and a port the player worker
 * posts samples to.
 */
export class AudioPlayout {
  private node: AudioWorkletNode | null = null;
  private gain: GainNode;
  private timer: ReturnType<typeof setInterval>;
  private last: AudioOutput;
  private closed = false;

  private constructor(
    private readonly ctx: AudioContext,
    private readonly video: HTMLMediaElement,
    private readonly changed: (output: AudioOutput) => void
  ) {
    this.gain = ctx.createGain();
    this.gain.connect(ctx.destination);
    this.volume();
    video.addEventListener("volumechange", this.volume);
    ctx.addEventListener("statechange", this.poll);
    this.timer = setInterval(this.poll, LATENCY_POLL_MS);
    this.last = this.output();
  }

  // Opens an output on `ctx`, answering it and the port for the worker;
  // `changed` hears of the output's later changes.
  static async open(
    ctx: AudioContext,
    video: HTMLMediaElement,
    changed: (output: AudioOutput) => void
  ): Promise<{ playout: AudioPlayout; port: MessagePort }> {
    await loadModule(ctx);
    const playout = new AudioPlayout(ctx, video, changed);
    const node = new AudioWorkletNode(ctx, WORKLET_NAME, {
      numberOfInputs: 0,
      numberOfOutputs: 1,
      outputChannelCount: [2],
    });
    node.connect(playout.gain);
    playout.node = node;
    const channel = new MessageChannel();
    node.port.postMessage({ port: channel.port1 }, [channel.port1]);
    return { playout, port: channel.port2 };
  }

  output(): AudioOutput {
    return {
      sampleRate: this.ctx.sampleRate,
      running: this.ctx.state === "running",
      latency: (this.ctx.baseLatency || 0) + (this.ctx.outputLatency || 0),
    };
  }

  close(): void {
    if (this.closed) return;
    this.closed = true;
    clearInterval(this.timer);
    this.video.removeEventListener("volumechange", this.volume);
    this.ctx.removeEventListener("statechange", this.poll);
    this.node?.port.postMessage("stop");
    this.node?.disconnect();
    this.gain.disconnect();
  }

  private volume = () => {
    this.gain.gain.value = this.video.muted ? 0 : this.video.volume;
  };

  private poll = () => {
    const o = this.output();
    if (
      o.running !== this.last.running ||
      o.sampleRate !== this.last.sampleRate ||
      Math.abs(o.latency - this.last.latency) > 0.002
    ) {
      this.last = o;
      this.changed(o);
    }
  };
}
