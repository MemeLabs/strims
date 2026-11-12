// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The storybook's device frames. Imported by the frame's preload, before
// any app module evaluates, so this imports nothing.

export type Device = "pc" | "portrait" | "landscape";

export interface DeviceShape {
  width: number;
  height: number;
  /** As window.orientation reads it. */
  orientation: number;
}

// The story frame's viewport: pc's (null) fills the storybook page's
// content area (hack/storybook/check.py's VIEWPORTS has a size for it).
export const DEVICES: Record<Device, DeviceShape | null> = {
  pc: null,
  portrait: { width: 390, height: 844, orientation: 0 },
  landscape: { width: 844, height: 390, orientation: 90 },
};

export const isDevice = (v: string | null): v is Device =>
  v !== null && Object.keys(DEVICES).includes(v);

// What a phone frame's navigator.userAgent reads, so lib/userAgent (and the
// libraries that sniff it) see a phone: an iPhone's Safari.
export const PHONE_USER_AGENT =
  "Mozilla/5.0 (iPhone; CPU iPhone OS 18_5 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.5 Mobile/15E148 Safari/604.1";
