// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** A block of samples the worklet posts: the index of its first sample in
 * the processor's own count, and its channels' samples one after another.
 */
export interface WorkletBlock {
  sample: number;
  data: Float32Array<ArrayBuffer>;
}

/**
 * The capture's AudioWorklet: gathers each input channel into blocks of
 * `frames` samples and posts them (WorkletBlock) with the index of their
 * first sample, counted by the processor from its first input. The
 * context's `currentFrame` can't stand in for that count: an engine may
 * report 0 for a new processor's first render quantum and the context's
 * running frame from the next on, which would put the first block the
 * context's age before the rest.
 */
export const WORKLET_SOURCE = `
registerProcessor("strims-capture", class extends AudioWorkletProcessor {
  constructor(options) {
    super();
    const { channels, frames } = options.processorOptions;
    this.channels = channels;
    this.frames = frames;
    this.buf = Array.from({ length: channels }, () => new Float32Array(frames));
    this.fill = 0;
    this.count = 0;
    this.stopped = false;
    this.port.onmessage = () => (this.stopped = true);
  }
  process(inputs) {
    if (this.stopped) return false;
    const input = inputs[0] || [];
    const n = input[0] ? input[0].length : 128;
    for (let i = 0; i < n; ) {
      const take = Math.min(n - i, this.frames - this.fill);
      for (let c = 0; c < this.channels; c++) {
        const src = input[Math.min(c, input.length - 1)];
        if (src) this.buf[c].set(src.subarray(i, i + take), this.fill);
        else this.buf[c].fill(0, this.fill, this.fill + take);
      }
      this.fill += take;
      i += take;
      if (this.fill === this.frames) {
        const data = new Float32Array(this.frames * this.channels);
        this.buf.forEach((b, c) => data.set(b, c * this.frames));
        this.port.postMessage({ sample: this.count, data }, [data.buffer]);
        this.count += this.frames;
        this.fill = 0;
      }
    }
    return true;
  }
});
`;
