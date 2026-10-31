// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { playsWireless } from "./airplay";

/** Milliseconds between the trial's looks at the video. */
export const TRIAL_TICK_MS = 250;
/** Seconds of media buffered after which the video should have drawn a frame. */
export const TRIAL_BUFFERED = 1;
/** Milliseconds the video may hold TRIAL_BUFFERED of media without drawing a frame. */
export const TRIAL_FRAME_MS = 5000;
/** Milliseconds the video may go without TRIAL_BUFFERED of media or a frame. */
export const TRIAL_MEDIA_MS = 10_000;
/**
 * The most milliseconds one look counts: a page whose timers were held
 * back, such as one in the background, counts no more than this between
 * two of them.
 */
const MAX_TICK_MS = 2 * TRIAL_TICK_MS;

/**
 * Why a rendition failed its trial: the video failed, it drew no frame of
 * the media it holds, or no media came. The first two say the browser
 * can't decode the rendition; the last says nothing of its codec.
 */
export type TrialFailure = "error" | "frames" | "media";

export interface TrialEvents {
  /** The video drew a frame of the rendition. */
  confirmed: () => void;
  failed: (reason: TrialFailure, message: string) => void;
}

/** What the trial reads of the video. */
export type TrialVideo = Pick<
  HTMLVideoElement,
  "addEventListener" | "removeEventListener" | "dispatchEvent" | "buffered" | "error"
> &
  Partial<Pick<HTMLVideoElement, "requestVideoFrameCallback" | "cancelVideoFrameCallback">>;

/** The clock and the page state the trial counts time by. */
export interface TrialScope {
  /** Milliseconds, on any clock that only goes forward. */
  now: () => number;
  /** The page shows: frames are drawn only then. */
  visible: () => boolean;
  /** The video plays here, not on a receiver. */
  local: (video: TrialVideo) => boolean;
  setInterval: (f: () => void, ms: number) => unknown;
  clearInterval: (handle: unknown) => void;
}

const windowScope = (): TrialScope => ({
  now: () => performance.now(),
  visible: () => document.visibilityState !== "hidden",
  local: (video) => !playsWireless(video),
  setInterval: (f, ms) => setInterval(f, ms),
  clearInterval: (h) => clearInterval(h as ReturnType<typeof setInterval>),
});

const bufferedSeconds = (ranges: TimeRanges): number => {
  let total = 0;
  for (let i = 0; i < ranges.length; i++) {
    total += ranges.end(i) - ranges.start(i);
  }
  return total;
};

/**
 * Whether a rendition plays here, as its video shows it: confirmed once
 * the video draws a frame (requestVideoFrameCallback, which also answers
 * a paused video's first frame), and failed when the video fails, when
 * it holds TRIAL_BUFFERED of media for TRIAL_FRAME_MS without drawing a
 * frame, or when it goes TRIAL_MEDIA_MS with neither. Time counts only
 * while the page shows and the video plays here, since no frame is drawn
 * otherwise. Where the video tells no frames, a rendition is confirmed
 * once it held its media for TRIAL_FRAME_MS without failing.
 *
 * It answers once, then stops looking; `destroy` stops it before.
 */
export class RenditionTrial {
  private done = false;
  private frame: number | null = null;
  private readonly frames: boolean;
  private readonly timer: unknown;
  private last: number;
  /** Milliseconds counted without TRIAL_BUFFERED of media, and with it. */
  private waiting = 0;
  private holding = 0;

  constructor(
    private readonly video: TrialVideo,
    private readonly events: TrialEvents,
    private readonly scope: TrialScope = windowScope()
  ) {
    video.addEventListener("error", this.onError);
    this.frames = typeof video.requestVideoFrameCallback === "function";
    if (this.frames) {
      this.frame = video.requestVideoFrameCallback(this.onFrame);
    }
    this.last = scope.now();
    this.timer = scope.setInterval(this.tick, TRIAL_TICK_MS);
  }

  destroy(): void {
    if (this.done) return;
    this.done = true;
    this.scope.clearInterval(this.timer);
    this.video.removeEventListener("error", this.onError);
    if (this.frame !== null) {
      this.video.cancelVideoFrameCallback?.(this.frame);
      this.frame = null;
    }
  }

  private confirm(): void {
    if (this.done) return;
    this.destroy();
    this.events.confirmed();
  }

  private fail(reason: TrialFailure, message: string): void {
    if (this.done) return;
    this.destroy();
    this.events.failed(reason, message);
  }

  private onFrame = () => {
    this.frame = null;
    this.confirm();
  };

  private onError = () => {
    const e = this.video.error;
    this.fail("error", `the video failed: ${e?.message || e?.code || "unknown error"}`);
  };

  private tick = () => {
    if (this.done) return;
    const now = this.scope.now();
    const elapsed = Math.min(Math.max(now - this.last, 0), MAX_TICK_MS);
    this.last = now;
    if (!this.scope.visible() || !this.scope.local(this.video)) return;
    if (bufferedSeconds(this.video.buffered) < TRIAL_BUFFERED) {
      this.waiting += elapsed;
      if (this.waiting >= TRIAL_MEDIA_MS) {
        this.fail("media", `no ${TRIAL_BUFFERED} s of media came in ${TRIAL_MEDIA_MS / 1000} s`);
      }
      return;
    }
    this.holding += elapsed;
    if (this.holding < TRIAL_FRAME_MS) return;
    if (this.frames) {
      this.fail("frames", `no frame drawn of the media held for ${TRIAL_FRAME_MS / 1000} s`);
    } else {
      this.confirm();
    }
  };
}
