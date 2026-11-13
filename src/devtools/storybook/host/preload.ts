// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The devtools entry's first import: in a storybook frame document it
// makes this realm the device's before any app module evaluates. The app
// reads the device type from the user agent once, at load (lib/userAgent:
// SwipablePanel, ChatBar, Footer or Header, ...), so a device frame reports
// its device's user agent, window.orientation and screen.orientation as the
// device has them, and a tablet's touch screen as its browser does.

import { DEVICES, Device, DeviceTouch, isDevice } from "./devices";
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

// The navigator's touch screen, and the touch events' handler properties a
// touch screen's browser has (feature checks read `"ontouchstart" in window`).
const fakeTouch = ({ maxTouchPoints, platform, standalone }: DeviceTouch) => {
  const props: Record<string, unknown> = { maxTouchPoints, platform };
  if (standalone !== undefined) {
    props.standalone = standalone;
  }
  for (const [name, value] of Object.entries(props)) {
    Object.defineProperty(navigator, name, { get: () => value, configurable: true });
  }
  for (const target of [window, document]) {
    for (const name of ["ontouchstart", "ontouchmove", "ontouchend", "ontouchcancel"]) {
      if (!(name in target)) {
        Object.defineProperty(target, name, { value: null, writable: true, configurable: true });
      }
    }
  }
};

// screen.orientation's type for a screen of `width` by `height` turned
// `angle` degrees from its natural orientation, as Chromium names it: the
// natural one is primary, a phone's portrait or an Android tablet's
// landscape.
const orientationType = (width: number, height: number, angle: number): OrientationType => {
  const tall = angle % 180 ? width > height : height > width;
  const types: OrientationType[] = tall
    ? ["portrait-primary", "landscape-primary", "portrait-secondary", "landscape-secondary"]
    : ["landscape-primary", "portrait-secondary", "landscape-secondary", "portrait-primary"];
  return types[angle / 90];
};

let orientation = shape?.orientation;
if (shape) {
  const angle = () => (orientation + 360) % 360;
  Object.defineProperty(navigator, "userAgent", {
    get: () => shape.userAgent,
    configurable: true,
  });
  Object.defineProperty(window, "orientation", {
    get: () => orientation,
    configurable: true,
  });
  Object.defineProperty(screen.orientation, "angle", { get: angle, configurable: true });
  Object.defineProperty(screen.orientation, "type", {
    get: () => orientationType(shape.width, shape.height, angle()),
    configurable: true,
  });
  if (shape.touch) {
    fakeTouch(shape.touch);
  }
}

// A device frame's window.orientation (and screen.orientation's) from now
// on: StoryView turns a landscape frame the other way round
// (Story.landscape) before its story renders.
export const setFrameOrientation = (value: number): void => {
  orientation = value;
};
