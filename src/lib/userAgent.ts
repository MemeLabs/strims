// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { UAParser } from "ua-parser-js";

export const enum DeviceType {
  TV = "tv",
  Portable = "portable",
  PC = "pc",
}

// ua-parser-js's device types; any other (none) is a PC.
const deviceTypes: Partial<Record<UAParser.DeviceTypes, DeviceType>> = {
  "console": DeviceType.TV,
  "smarttv": DeviceType.TV,
  "mobile": DeviceType.Portable,
  "tablet": DeviceType.Portable,
  "wearable": DeviceType.Portable,
  "embedded": DeviceType.Portable,
  // Headsets (Quest, Vision Pro): 1.x called them wearables.
  "xr": DeviceType.Portable,
};

// iPadOS's Safari asks for desktop sites by default, naming itself a Mac,
// which has no touch screen.
const isIPadOS = (os: string): boolean => os === "macOS" && navigator.maxTouchPoints > 1;

export const [DEVICE_TYPE, OS] = ((): [DeviceType, string] => {
  const ua = new UAParser();
  const os = ua.getOS().name;
  const type = isIPadOS(os) ? DeviceType.Portable : deviceTypes[ua.getDevice().type];
  return [type ?? DeviceType.PC, os];
})();

export const IS_PWA = !window.matchMedia("(display-mode: browser)").matches;

export const IS_IOS = OS === "iOS" || isIPadOS(OS);
export const IS_ANDROID = OS === "Android";
