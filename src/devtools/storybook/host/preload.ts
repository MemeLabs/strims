// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The devtools entry's first import: in a storybook frame document it
// makes this realm the device's before any app module evaluates. The app
// reads the device type from the user agent once, at load (lib/userAgent:
// SwipablePanel, ChatBar, Footer or Header, ...), so a phone frame reports
// a phone's user agent, and window.orientation as a phone has it.

import { DEVICES, Device, PHONE_USER_AGENT, isDevice } from "./devices";
import { FRAME_ROOT, INDEX_ROUTE, ROOT, Theme } from "./protocol";

export type FrameDocument =
  | {
      kind: "story";
      /** The story's route on the page: /storybook/<module path>[/<name>]. */
      route: string;
      theme: Theme;
      device: Device;
    }
  | { kind: "index" };

const parse = (): FrameDocument | null => {
  const hash = location.hash.replace(/^#/, "");
  const q = hash.indexOf("?");
  const pathname = q < 0 ? hash : hash.substring(0, q);
  const params = new URLSearchParams(q < 0 ? "" : hash.substring(q + 1));
  if (pathname === INDEX_ROUTE) {
    return { kind: "index" };
  }
  if (!pathname.startsWith(FRAME_ROOT + "/")) {
    return null;
  }
  const device = params.get("device");
  return {
    kind: "story",
    route: ROOT + pathname.substring(FRAME_ROOT.length),
    theme: params.get("theme") === "light" ? "light" : "dark",
    device: isDevice(device) ? device : "pc",
  };
};

/** This document's storybook role, if it is a storybook frame. */
export const frameDocument = parse();

const shape = frameDocument?.kind === "story" ? DEVICES[frameDocument.device] : null;
if (shape) {
  Object.defineProperty(navigator, "userAgent", {
    get: () => PHONE_USER_AGENT,
    configurable: true,
  });
  Object.defineProperty(window, "orientation", {
    get: () => shape.orientation,
    configurable: true,
  });
}
