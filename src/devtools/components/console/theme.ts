// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/**
 * The console's chart look, a Vega-Lite config every saved view's spec is embedded with: recessive
 * axes and grid, the system sans, and fixed categorical, sequential and status colours.
 */

/** Categorical slots, in order (never cycled: a ninth series folds or facets). */
export const SERIES = [
  "#2a78d6",
  "#eb6834",
  "#1baf7a",
  "#eda100",
  "#e87ba4",
  "#008300",
  "#6250d6",
  "#e34948",
];

/** Status colours, for states (healthy, ramp, collapse), never for series. */
export const STATUS = {
  good: "#0ca30c",
  warning: "#fab219",
  serious: "#ec835a",
  critical: "#d03b3b",
  muted: "#c3c2b7",
  quiet: "#e9e8e3",
};

/** The sequential ramp (blue, light to dark). */
export const SEQUENTIAL = [
  "#cde2fb",
  "#9ec5f4",
  "#6da7ec",
  "#3987e5",
  "#256abf",
  "#184f95",
  "#0d366b",
];

const INK = "#0b0b0b";
const INK_2 = "#52514e";
const MUTED = "#898781";
const GRID = "#e1e0d9";
const AXIS = "#c3c2b7";
// Vega measures text on a canvas and draws it in SVG: a named face resolves alike in both, where
// system-ui needn't.
const FONT = 'Helvetica, Arial, "Liberation Sans", sans-serif';

export const config = {
  background: "#fcfcfb",
  font: FONT,
  padding: 8,
  view: { stroke: null },
  title: { color: INK, fontSize: 13, fontWeight: 600, anchor: "start", offset: 8 },
  axis: {
    domainColor: AXIS,
    gridColor: GRID,
    tickColor: AXIS,
    labelColor: MUTED,
    titleColor: INK_2,
    titleFontWeight: 500,
    labelFontSize: 11,
    titleFontSize: 11,
  },
  axisY: { minExtent: 48 },
  legend: { labelColor: INK_2, titleColor: INK_2, labelFontSize: 11, titleFontSize: 11 },
  header: { labelColor: INK_2, titleColor: INK_2, labelFontSize: 11 },
  range: { category: SERIES, ramp: SEQUENTIAL, heatmap: SEQUENTIAL },
  line: { strokeWidth: 2 },
  point: { size: 64, filled: true },
  bar: { cornerRadiusEnd: 2 },
  concat: { spacing: 14 },
};
