// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The app's form styles, which the page's buttons use and don't load.
import "../../../components/Form/Form.scss";
import "../../../components/Form/Button.scss";

import React, { ReactNode, useLayoutEffect, useState } from "react";

import Broadcast from "../../../pages/Broadcast";
import { Story } from "../../storybook";

// Hides the browser's screen capture while mounted, as a browser without
// getDisplayMedia (most mobile ones) has none; the page under it renders
// once it's gone.
const NoScreenCapture: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [hidden, setHidden] = useState(false);
  useLayoutEffect(() => {
    const devices = navigator.mediaDevices;
    if (devices) {
      Object.defineProperty(devices, "getDisplayMedia", { value: undefined, configurable: true });
    }
    setHidden(true);
    return () => void delete (devices as { getDisplayMedia?: unknown })?.getDisplayMedia;
  }, []);
  return hidden ? <>{children}</> : null;
};

// Only the idle page: a start opens the camera or a screen share.
const stories: Story[] = [
  {
    name: "Idle",
    component: () => <Broadcast />,
  },
  {
    name: "Camera only",
    component: () => (
      <NoScreenCapture>
        <Broadcast />
      </NoScreenCapture>
    ),
  },
];

export default stories;
