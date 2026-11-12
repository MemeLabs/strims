// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/Debugger/Debugger.scss";
import "./Debugger.stories.scss";

import clsx from "clsx";
import React, { ReactNode, useMemo, useRef } from "react";

import {
  ActivityChart,
  MockStreamChart,
  RTCChart,
  SwarmChart,
} from "../../../../components/Debugger/ActivityChart";
import Portlet from "../../../../components/Debugger/Portlet";
import { Story } from "../../../storybook";
import { MetricsOptions, metricsHistory } from "./metrics";
import { PortletPlacement, usePortletInFrame } from "./portlet";
import { useFakePeerConnections } from "./rtc";

// The charts size themselves by the portlet they're in.
const InPortlet: React.FC<PortletPlacement & { children: ReactNode }> = ({
  children,
  ...placement
}) => {
  const ref = useRef<HTMLDivElement>(null);
  usePortletInFrame(() => ref.current, placement);
  return (
    <div className={clsx("debugger_story", { "debugger_story--large": placement.height > 600 })}>
      <Portlet isOpen onClose={() => undefined}>
        <div ref={ref}>{children}</div>
      </Portlet>
    </div>
  );
};

const LARGE: PortletPlacement = { width: 720, height: 700 };

type Chart = typeof ActivityChart;

const ChartStory: React.FC<{
  chart: Chart;
  placement?: PortletPlacement;
  metrics?: MetricsOptions;
}> = ({ chart: C, placement = LARGE, metrics }) => {
  const history = useMemo(() => metricsHistory(metrics), []);
  return (
    <InPortlet {...placement}>
      <C history={history} />
    </InPortlet>
  );
};

const RTCStory: React.FC = () => {
  useFakePeerConnections();
  return (
    <InPortlet {...LARGE}>
      <RTCChart />
    </InPortlet>
  );
};

const RTCEmptyStory: React.FC = () => (
  <InPortlet>
    <RTCChart />
  </InPortlet>
);

const stories: Story[] = [
  {
    name: "Activity",
    component: () => <ChartStory chart={ActivityChart} />,
  },
  {
    name: "Activity small",
    component: () => <ChartStory chart={ActivityChart} placement={{}} />,
  },
  {
    name: "Activity no memory",
    component: () => <ChartStory chart={ActivityChart} metrics={{ noMemory: true }} />,
  },
  {
    name: "Activity few samples",
    component: () => <ChartStory chart={ActivityChart} metrics={{ count: 3 }} />,
  },
  {
    name: "Swarms",
    component: () => <ChartStory chart={SwarmChart} />,
  },
  {
    name: "Swarms small",
    component: () => <ChartStory chart={SwarmChart} placement={{}} />,
  },
  {
    name: "Mock streams",
    component: () => <ChartStory chart={MockStreamChart} />,
  },
  {
    name: "RTC",
    component: () => <RTCStory />,
  },
  {
    name: "RTC no connections",
    component: () => <RTCEmptyStory />,
  },
];

export default stories;
