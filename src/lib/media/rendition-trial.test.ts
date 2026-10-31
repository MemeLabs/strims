// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  RenditionTrial,
  TRIAL_FRAME_MS,
  TRIAL_MEDIA_MS,
  TRIAL_TICK_MS,
  TrialFailure,
  TrialScope,
  TrialVideo,
} from "./rendition-trial";

class FakeVideo extends EventTarget {
  ranges: [number, number][] = [];
  error: { code: number; message: string } | null = null;
  callbacks = new Map<number, () => void>();
  cancelled: number[] = [];
  private next = 1;

  get buffered() {
    return {
      length: this.ranges.length,
      start: (i: number) => this.ranges[i][0],
      end: (i: number) => this.ranges[i][1],
    };
  }

  requestVideoFrameCallback(cb: () => void): number {
    const id = this.next++;
    this.callbacks.set(id, cb);
    return id;
  }

  cancelVideoFrameCallback(id: number): void {
    this.cancelled.push(id);
    this.callbacks.delete(id);
  }

  // Draws a frame: the callbacks asked for it run.
  draw(): void {
    const cbs = [...this.callbacks.values()];
    this.callbacks.clear();
    for (const cb of cbs) cb();
  }

  fail(code: number, message: string): void {
    this.error = { code, message };
    this.dispatchEvent(new Event("error"));
  }
}

// A trial on a fake video and clock, whose ticks the test runs.
const trial = (video = new FakeVideo()) => {
  let now = 0;
  let tick: (() => void) | null = null;
  const state = { visible: true, local: true };
  const scope: TrialScope = {
    now: () => now,
    visible: () => state.visible,
    local: () => state.local,
    setInterval: (f) => ((tick = f), 1),
    clearInterval: () => (tick = null),
  };
  const answers: (["confirmed"] | ["failed", TrialFailure, string])[] = [];
  const t = new RenditionTrial(
    video as unknown as TrialVideo,
    {
      confirmed: () => answers.push(["confirmed"]),
      failed: (reason, message) => answers.push(["failed", reason, message]),
    },
    scope
  );
  // Runs the trial's looks over `ms`.
  const run = (ms: number) => {
    for (let at = 0; at < ms; at += TRIAL_TICK_MS) {
      now += TRIAL_TICK_MS;
      tick?.();
    }
  };
  const reasons = () => answers.map((a) => (a[0] === "failed" ? a[1] : a[0]));
  return {
    t,
    video,
    run,
    answers,
    reasons,
    state,
    stopped: () => tick === null,
    skip: (ms: number) => (now += ms),
  };
};

describe("RenditionTrial", () => {
  it("confirms a rendition once the video draws a frame, and stops looking", () => {
    const { video, run, reasons, stopped } = trial();
    run(1000);
    video.ranges = [[0, 2]];
    video.draw();
    assert.deepStrictEqual(reasons(), ["confirmed"]);
    assert.ok(stopped());
    video.fail(3, "late");
    run(TRIAL_MEDIA_MS * 2);
    assert.deepStrictEqual(reasons(), ["confirmed"]);
  });

  it("fails a rendition whose video fails", () => {
    const { video, answers, stopped } = trial();
    video.fail(3, "PIPELINE_ERROR_DECODE");
    assert.deepStrictEqual(answers, [
      ["failed", "error", "the video failed: PIPELINE_ERROR_DECODE"],
    ]);
    assert.ok(stopped());
    assert.strictEqual(video.callbacks.size, 0);
    assert.deepStrictEqual(video.cancelled, [1]);
  });

  it("fails a rendition whose media the video holds without drawing a frame", () => {
    const { video, run, reasons } = trial();
    video.ranges = [[0, 0.6]];
    run(TRIAL_MEDIA_MS - 2 * TRIAL_TICK_MS);
    video.ranges = [
      [0, 0.6],
      [0.7, 1.2],
    ];
    run(TRIAL_FRAME_MS - TRIAL_TICK_MS);
    assert.deepStrictEqual(reasons(), []);
    run(TRIAL_TICK_MS);
    assert.deepStrictEqual(reasons(), ["frames"]);
  });

  it("fails a rendition no media of which comes", () => {
    const { run, reasons } = trial();
    run(TRIAL_MEDIA_MS - TRIAL_TICK_MS);
    assert.deepStrictEqual(reasons(), []);
    run(TRIAL_TICK_MS);
    assert.deepStrictEqual(reasons(), ["media"]);
  });

  it("counts no time while the page is hidden or the video plays on a receiver", () => {
    const { video, run, reasons, state, skip } = trial();
    video.ranges = [[0, 5]];
    state.visible = false;
    run(TRIAL_FRAME_MS * 3);
    state.visible = true;
    state.local = false;
    run(TRIAL_FRAME_MS * 3);
    state.local = true;
    skip(TRIAL_FRAME_MS * 10);
    run(TRIAL_FRAME_MS - 2 * TRIAL_TICK_MS);
    assert.deepStrictEqual(reasons(), []);
    run(2 * TRIAL_TICK_MS);
    assert.deepStrictEqual(reasons(), ["frames"]);
  });

  it("confirms, where the video tells no frames, a rendition that held its media without failing", () => {
    const video = new FakeVideo();
    Object.defineProperty(video, "requestVideoFrameCallback", { value: undefined });
    const { run, reasons } = trial(video);
    run(TRIAL_MEDIA_MS - TRIAL_TICK_MS);
    video.ranges = [[0, 3]];
    run(TRIAL_FRAME_MS);
    assert.deepStrictEqual(reasons(), ["confirmed"]);
    const silent = new FakeVideo();
    Object.defineProperty(silent, "requestVideoFrameCallback", { value: undefined });
    const none = trial(silent);
    none.run(TRIAL_MEDIA_MS);
    assert.deepStrictEqual(none.reasons(), ["media"]);
  });

  it("answers nothing once destroyed", () => {
    const { t, video, run, reasons, stopped } = trial();
    t.destroy();
    assert.ok(stopped());
    assert.deepStrictEqual(video.cancelled, [1]);
    video.fail(3, "late");
    video.draw();
    run(TRIAL_MEDIA_MS);
    assert.deepStrictEqual(reasons(), []);
  });
});
