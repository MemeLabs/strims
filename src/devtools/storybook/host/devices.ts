// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The storybook's device frames. Imported by the frame's preload, before
// any app module evaluates, so this imports nothing.

export type Device =
  | "pc"
  | "portrait"
  | "landscape"
  | "ipad_portrait"
  | "ipad_landscape"
  | "tablet_portrait"
  | "tablet_landscape";

// What a frame's navigator says about its touch screen. With one, the
// realm also has touch events' handler properties ("ontouchstart" in
// window and document), which use-gesture, react-select and swiper sniff.
export interface DeviceTouch {
  maxTouchPoints: number;
  platform: string;
  /** iOS Safari's navigator.standalone (false in a tab); other browsers have none. */
  standalone?: boolean;
}

export interface DeviceShape {
  width: number;
  height: number;
  /**
   * As window.orientation reads it: turned from the device's natural
   * orientation, a phone's or an iPad's portrait, but most Android
   * tablets' landscape.
   */
  orientation: number;
  /** What its navigator.userAgent reads: the device's browser's own. */
  userAgent: string;
  /** None: the realm's own navigator (the phone frames have kept it). */
  touch?: DeviceTouch;
}

// window.orientation of a phone on its side, either way round, named as
// the layout's classes name them (layout--landscape_ccw, layout--landscape_cw).
// The landscape phone frame is turned ccw unless its story says cw
// (Story.landscape); the tablets' are never turned.
export const LANDSCAPE_ORIENTATIONS = { ccw: 90, cw: -90 } as const;

// An iPhone's Safari, so lib/userAgent (and the libraries that sniff it)
// see a phone.
export const PHONE_USER_AGENT =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1";

// iPadOS Safari asks for desktop sites by default: a Mac's user agent, the
// iPad told apart only by its touch screen (lib/userAgent's parse takes it
// for a pc).
export const IPAD_USER_AGENT =
  "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Safari/605.1.15";
const IPAD_TOUCH: DeviceTouch = { maxTouchPoints: 5, platform: "MacIntel", standalone: false };

// Chrome on an Android tablet: its reduced user agent, Android without
// Mobile, which ua-parser-js calls a tablet.
export const ANDROID_TABLET_USER_AGENT =
  "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/141.0.0.0 Safari/537.36";
const ANDROID_TABLET_TOUCH: DeviceTouch = { maxTouchPoints: 5, platform: "Linux armv8l" };

// The story frame's viewport: pc's (null) fills the storybook page's
// content area (hack/storybook/check.py's VIEWPORTS has a size for it).
// The tablets are an iPad Air's (iPadOS Safari, a pc to the app) and a
// 16:10 Android tablet's, a Pixel Tablet's or a Galaxy Tab S9's (natural
// landscape, so window.orientation is 0 in landscape).
export const DEVICES: Record<Device, DeviceShape | null> = {
  pc: null,
  portrait: { width: 390, height: 844, orientation: 0, userAgent: PHONE_USER_AGENT },
  landscape: {
    width: 844,
    height: 390,
    orientation: LANDSCAPE_ORIENTATIONS.ccw,
    userAgent: PHONE_USER_AGENT,
  },
  ipad_portrait: {
    width: 820,
    height: 1180,
    orientation: 0,
    userAgent: IPAD_USER_AGENT,
    touch: IPAD_TOUCH,
  },
  ipad_landscape: {
    width: 1180,
    height: 820,
    orientation: 90,
    userAgent: IPAD_USER_AGENT,
    touch: IPAD_TOUCH,
  },
  tablet_portrait: {
    width: 800,
    height: 1280,
    orientation: 90,
    userAgent: ANDROID_TABLET_USER_AGENT,
    touch: ANDROID_TABLET_TOUCH,
  },
  tablet_landscape: {
    width: 1280,
    height: 800,
    orientation: 0,
    userAgent: ANDROID_TABLET_USER_AGENT,
    touch: ANDROID_TABLET_TOUCH,
  },
};

export const isDevice = (v: string | null): v is Device =>
  v !== null && Object.keys(DEVICES).includes(v);
