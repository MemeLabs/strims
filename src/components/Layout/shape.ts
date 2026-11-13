// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { LayoutContextProps } from "../../contexts/Layout";

// The screen's orientation as the layout's classes name it: 0 in portrait,
// and in landscape 90 turned counterclockwise (layout--landscape_ccw) or -90
// clockwise (layout--landscape_cw), as window.orientation reads it on a
// device whose natural orientation is portrait. Most Android tablets' is
// landscape, where window.orientation reads 0 in landscape and 90 or -90 in
// portrait, so screen.orientation's type says which of the two the screen
// is, and such a tablet's natural landscape counts as turned
// counterclockwise. Browsers with no window.orientation (desktops) are 0.
export const screenOrientation = (): number => {
  const turn = window.orientation;
  const type = screen.orientation?.type;
  if (turn === undefined || !type) {
    return turn ?? 0;
  }
  if (type.startsWith("portrait")) {
    return 0;
  }
  return turn === 90 || turn === -90 ? turn : turn === 0 ? 90 : -90;
};

// The layout's classes that follow the viewport's size and orientation
// (`orientation` as screenOrientation reads it), apart so the devtools
// storybook's device frames carry the same ones.
export const layoutShapeClassNames = (
  width: number,
  height: number,
  orientation: number
): Record<string, boolean> => {
  const aspectRatio = width / height;
  return {
    "layout--portrait": orientation === 0,
    "layout--landscape_ccw": orientation === 90,
    "layout--landscape_cw": orientation === -90,
    "layout--min_aspect_ratio_0_6": aspectRatio >= 0.6,
    "layout--min_aspect_ratio_1_2": aspectRatio >= 1.2,
    "layout--min_aspect_ratio_2": aspectRatio >= 2,
    "layout--min_aspect_ratio_4": aspectRatio >= 3,
    "layout--max_aspect_ratio_0_6": aspectRatio < 0.6,
    "layout--max_aspect_ratio_1_2": aspectRatio < 1.2,
    "layout--max_aspect_ratio_2": aspectRatio < 2,
    "layout--max_aspect_ratio_4": aspectRatio < 3,
    "layout--min_width_sm": width >= 576,
    "layout--min_width_md": width >= 768,
    "layout--min_width_lg": width >= 992,
    "layout--min_width_xl": width >= 1200,
    "layout--max_width_sm": width < 576,
    "layout--max_width_md": width < 768,
    "layout--max_width_lg": width < 992,
    "layout--max_width_xl": width < 1200,
  };
};

// The layout's classes that follow its context's state.
export const layoutStateClassNames = (layout: LayoutContextProps): Record<string, boolean> => ({
  "layout--meme_open": layout.overlayState.open,
  "layout--swap": layout.swapMainPanels,
  "layout--show_chat": layout.showChat,
  "layout--show_video": layout.showVideo,
  "layout--full_screen_chat": layout.fullScreenChat,
  "layout--theater_mode": layout.theaterMode,
  "layout--expand_nav": layout.expandNav,
});
