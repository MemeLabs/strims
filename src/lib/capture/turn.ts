// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Turning a capture's frames upright.

// Where a frame of `width` by `height` lands turned by `quarterTurns`
// clockwise: the canvas's size, and the transform (a, b, c, d, e, f, as
// CanvasRenderingContext2D.setTransform takes it) that draws the frame at
// the origin turned onto it.
export const turnedDraw = (
  width: number,
  height: number,
  quarterTurns: number
): { size: [number, number]; transform: [number, number, number, number, number, number] } => {
  switch (((quarterTurns % 4) + 4) % 4) {
    case 1:
      return { size: [height, width], transform: [0, 1, -1, 0, height, 0] };
    case 2:
      return { size: [width, height], transform: [-1, 0, 0, -1, width, height] };
    case 3:
      return { size: [height, width], transform: [0, -1, 1, 0, 0, width] };
    default:
      return { size: [width, height], transform: [1, 0, 0, 1, 0, 0] };
  }
};

/**
 * The quarter turns clockwise that put upright the frames of a camera held
 * so the page is upright, whose frames come turned from the track's
 * settings: the sensor's landscape picture, a quarter turn anticlockwise of
 * the portrait the settings give.
 */
export const UPRIGHT_TURNS = 1;
