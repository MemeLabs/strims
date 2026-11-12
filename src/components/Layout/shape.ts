// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { LayoutContextProps } from "../../contexts/Layout";

// The layout's classes that follow the viewport's size and orientation
// (`orientation` as window.orientation reads it: 0, 90 or -90), apart so the
// devtools storybook's device frames carry the same ones.
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
