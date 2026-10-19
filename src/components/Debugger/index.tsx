// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Debugger.scss";

import clsx from "clsx";
import React, { useState } from "react";

import { ActivityChart, MockStreamChart, RTCChart, SwarmChart } from "./ActivityChart";
import Portlet, { PortletProps } from "./Portlet";
import { latest } from "./series";
import Snapshot from "./Snapshot";
import useMetricsHistory from "./useMetricsHistory";

type Tab = "activity" | "swarms" | "mock streams" | "rtc" | "snapshot";

const TABS: Tab[] = ["activity", "swarms", "mock streams", "rtc", "snapshot"];

interface DebuggerContentProps {
  selectedTab: Tab;
  onSelectTab: (tab: Tab) => void;
}

// The portlet mounts its children only while it's open, so the metrics are
// watched only then.
const DebuggerContent: React.FC<DebuggerContentProps> = ({ selectedTab, onSelectTab }) => {
  const { snapshots, error } = useMetricsHistory();

  const content = (() => {
    if (selectedTab === "rtc") {
      return <RTCChart />;
    }
    if (snapshots.length === 0) {
      return <div className="debugger__status">{error ?? "waiting for metrics"}</div>;
    }
    switch (selectedTab) {
      case "activity":
        return <ActivityChart history={snapshots} />;
      case "swarms":
        return <SwarmChart history={snapshots} />;
      case "mock streams":
        return <MockStreamChart history={snapshots} />;
      case "snapshot":
        return <Snapshot snapshot={latest(snapshots)} />;
    }
  })();

  return (
    <>
      <div className="debugger__tabs">
        {TABS.map((tab) => (
          <button
            key={tab}
            className={clsx("debugger__tabs__tab", {
              "debugger__tabs__tab--selected": tab === selectedTab,
            })}
            onClick={() => onSelectTab(tab)}
          >
            {tab}
          </button>
        ))}
      </div>
      {error && snapshots.length > 0 && <div className="debugger__status">{error}</div>}
      <div>{content}</div>
    </>
  );
};

type DebuggerProps = Omit<PortletProps, "children">;

const Debugger: React.FC<DebuggerProps> = ({ onClose, isOpen }) => {
  const [selectedTab, setSelectedTab] = useState<Tab>("activity");

  return (
    <Portlet onClose={onClose} isOpen={isOpen}>
      <DebuggerContent selectedTab={selectedTab} onSelectTab={setSelectedTab} />
    </Portlet>
  );
};

export default Debugger;
