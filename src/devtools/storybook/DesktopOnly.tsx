// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode } from "react";

import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";

export interface DesktopOnlyProps {
  /** Why nothing shows on phones, and what the app shows there instead. */
  note: ReactNode;
  children: ReactNode;
}

// The story of a component the app renders off portable devices alone: in a
// portable device's frame (a phone's or an Android tablet's, host/devices.ts)
// the note in its place.
export const DesktopOnly: React.FC<DesktopOnlyProps> = ({ note, children }) =>
  DEVICE_TYPE === DeviceType.Portable ? (
    <p className="storybook_device_note">{note}</p>
  ) : (
    <>{children}</>
  );
