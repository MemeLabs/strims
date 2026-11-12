// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The storybook's documents and what crosses between them. The page
// (#/storybook/<module path>/<name>) holds the nav, toolbar, catalog and
// error list; each story renders in an iframe of its own, a fresh
// devtools.html at #/storybook-frame/<module path>/<name>, sized to its
// device; the index (every story and the catalog's files) is built in a
// hidden one at #/storybook-index, so the page never evaluates a story
// module. Frames post FrameMessages to the page.
//
// Imported by the frame's preload, before any app module evaluates, so
// this imports types alone.

import type { IndexData } from "./catalog";
import type { Device } from "./devices";
import type { SessionSnapshot } from "./recorder";

export const ROOT = "/storybook";
export const FRAME_ROOT = "/storybook-frame";
export const INDEX_ROUTE = "/storybook-index";

export type Theme = "dark" | "light";

export interface FrameParams {
  theme: Theme;
  device: Device;
}

// The query the page and its frames share (theme=light, device=portrait|
// landscape), defaults left out.
export const frameQuery = ({ theme, device }: FrameParams): string => {
  const q = new URLSearchParams();
  if (theme !== "dark") {
    q.set("theme", theme);
  }
  if (device !== "pc") {
    q.set("device", device);
  }
  const s = q.toString();
  return s ? `?${s}` : "";
};

// A story route's (/storybook/<module path>/<name>) frame: the hash route
// to load in this document's page (devtools.html).
export const frameRoute = (route: string, params: FrameParams): string =>
  FRAME_ROOT + route.substring(ROOT.length) + frameQuery(params);

export type FrameMessage =
  | { storybook: "story"; snapshot: SessionSnapshot }
  | { storybook: "router"; path: string | null }
  | { storybook: "index"; index: IndexData }
  | { storybook: "index-error"; error: string };

export const isFrameMessage = (data: unknown): data is FrameMessage =>
  typeof data === "object" && data !== null && "storybook" in data;

// Posts to the page that holds this frame, if one does.
export const postToPage = (message: FrameMessage): void => {
  if (window.parent !== window) {
    window.parent.postMessage(message, location.origin);
  }
};
