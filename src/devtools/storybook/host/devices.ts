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

// Insets from a viewport's edges, in CSS px.
export interface Insets {
  top: number;
  right: number;
  bottom: number;
  left: number;
}

// What the device's screen hides of a page that fills it: its rounded
// corners and its notch, centred on the device's top edge, in CSS px.
export interface DeviceScreen {
  cornerRadius: number;
  notch?: {
    /** Along the top edge. */
    width: number;
    /** Into the screen from the top edge. */
    depth: number;
    /** Of its two corners inside the screen. */
    radius: number;
  };
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
  /**
   * env(safe-area-inset-*) as the device's Safari reports it for a page
   * that fills the screen (viewport-fit=cover). The styles read these
   * through --safe-area-inset-*, which the frame's preload sets
   * (webpack/SafeAreaInsets.ts). None means 0 on every side, as the
   * browser running the storybook reports.
   */
  safeArea?: Insets;
  /** None: nothing is drawn over the page. */
  screen?: DeviceScreen;
}

export type Turn = "ccw" | "cw";

// window.orientation of each landscape frame, either way round, named as
// the layout's classes name them (layout--landscape_ccw, layout--landscape_cw):
// a phone or an iPad on its side, turned from its natural portrait, and the
// Android tablet in its natural landscape or upside down. A landscape frame
// is turned ccw unless its story says cw (Story.landscape).
export const LANDSCAPE_ORIENTATIONS: Partial<Record<Device, Record<Turn, number>>> = {
  landscape: { ccw: 90, cw: -90 },
  ipad_landscape: { ccw: 90, cw: -90 },
  tablet_landscape: { ccw: 0, cw: 180 },
};

// The landscape phone frame's screen, an iPhone 13's or 14's (390 by 844),
// with its notch's width and depth approximate, and its safe area. iOS
// insets a landscape page by the notch's depth on both sides, whichever side
// the notch is turned to, and above the home indicator; the layout pads only
// the notch's side (layout--landscape_ccw, layout--landscape_cw). The
// portrait frame has neither: the page starts below the status bar (the app
// sets no apple-mobile-web-app-status-bar-style), so the notch never covers
// it, and its bottom inset is 0 above Safari's toolbar.
const IPHONE_SCREEN: DeviceScreen = {
  cornerRadius: 47,
  notch: { width: 162, depth: 33, radius: 20 },
};
const IPHONE_LANDSCAPE_SAFE_AREA: Insets = { top: 0, right: 47, bottom: 21, left: 47 };

// An iPhone's Safari, so lib/userAgent (and the libraries that sniff it)
// see a phone.
export const PHONE_USER_AGENT =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1";

// iPadOS Safari asks for desktop sites by default: a Mac's user agent, the
// iPad told apart only by its touch screen (as lib/userAgent tells it).
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
// The tablets are an iPad Air's (iPadOS Safari) and a
// 16:10 Android tablet's, a Pixel Tablet's or a Galaxy Tab S9's (natural
// landscape, so window.orientation is 0 in landscape).
export const DEVICES: Record<Device, DeviceShape | null> = {
  pc: null,
  portrait: { width: 390, height: 844, orientation: 0, userAgent: PHONE_USER_AGENT },
  landscape: {
    width: 844,
    height: 390,
    orientation: 90,
    userAgent: PHONE_USER_AGENT,
    safeArea: IPHONE_LANDSCAPE_SAFE_AREA,
    screen: IPHONE_SCREEN,
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
