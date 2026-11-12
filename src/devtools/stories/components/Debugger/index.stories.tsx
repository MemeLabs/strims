// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect } from "react";

import Debugger from "../../../../components/Debugger";
import { MockApi, Story } from "../../../storybook";
import { DebugService, MetricsMode } from "./metrics";
import { PortletPlacement, usePortletPlacement } from "./portlet";
import { useFakePeerConnections } from "./rtc";

const findTabs = () => document.querySelector(".debugger__tabs");

// Selects `tab` once the debugger shows its tabs.
const useSelectTab = (tab?: string) =>
  useEffect(() => {
    if (!tab) {
      return;
    }
    let frame = 0;
    const select = () => {
      const button = Array.from(
        document.querySelectorAll<HTMLElement>(".debugger__tabs__tab")
      ).find((el) => el.textContent === tab);
      if (button) {
        button.click();
      } else {
        frame = requestAnimationFrame(select);
      }
    };
    frame = requestAnimationFrame(select);
    return () => cancelAnimationFrame(frame);
  }, []);

interface DebuggerStoryProps extends PortletPlacement {
  mode?: MetricsMode;
  tab?: string;
}

const Placed: React.FC<DebuggerStoryProps> = ({ tab, ...placement }) => {
  usePortletPlacement(findTabs, placement);
  useSelectTab(tab);
  return null;
};

// The debugger's window opens at the page's top left, as in the app.
const DebuggerStory: React.FC<DebuggerStoryProps> = ({ mode = "history", ...props }) => (
  <MockApi services={() => ({ debug: new DebugService(mode) })}>
    <Debugger isOpen onClose={() => undefined} />
    <Placed {...props} />
  </MockApi>
);

const RTCStory: React.FC = () => {
  useFakePeerConnections();
  return <DebuggerStory tab="rtc" width={720} height={700} />;
};

const LARGE = { width: 720, height: 700 };

const stories: Story[] = [
  {
    name: "Activity",
    component: () => <DebuggerStory />,
  },
  {
    name: "Activity large",
    component: () => <DebuggerStory {...LARGE} />,
  },
  {
    name: "Swarms",
    component: () => <DebuggerStory tab="swarms" {...LARGE} />,
  },
  {
    name: "Mock streams",
    component: () => <DebuggerStory tab="mock streams" {...LARGE} />,
  },
  {
    name: "RTC",
    component: () => <RTCStory />,
  },
  {
    name: "Snapshot",
    component: () => <DebuggerStory tab="snapshot" />,
  },
  {
    name: "Waiting",
    component: () => <DebuggerStory mode="waiting" />,
  },
  {
    name: "Error",
    component: () => <DebuggerStory mode="error" />,
  },
];

export default stories;
