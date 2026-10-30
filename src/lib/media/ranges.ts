// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** A time range in seconds, start before end. */
export type Range = [number, number];

// Time ranges sorted by start, with overlapping and touching ones merged.
// Firefox can report a SourceBuffer's ranges out of order and overlapping.
export const normalizeRanges = (ranges: Range[]): Range[] => {
  const sorted = ranges
    .filter(([start, end]) => Number.isFinite(start) && Number.isFinite(end) && end > start)
    .sort((a, b) => a[0] - b[0]);
  const out: Range[] = [];
  for (const [start, end] of sorted) {
    const last = out[out.length - 1];
    if (last && start <= last[1]) {
      last[1] = Math.max(last[1], end);
    } else {
      out.push([start, end]);
    }
  }
  return out;
};

// A TimeRanges' ranges, normalized.
export const timeRanges = (tr: TimeRanges | null | undefined): Range[] => {
  const ranges: Range[] = [];
  for (let i = 0; i < (tr?.length ?? 0); i++) {
    ranges.push([tr.start(i), tr.end(i)]);
  }
  return normalizeRanges(ranges);
};
