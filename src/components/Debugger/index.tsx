// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Debugger.scss";

import PBReader from "@memelabs/protobuf/lib/pb/reader";
import React, { useState } from "react";
import { FiDownload } from "react-icons/fi";

import { MetricFamily } from "../../apis/io/prometheus/client/metrics";
import { MetricsFormat } from "../../apis/strims/debug/v1/debug";
import { useClient, useLazyCall } from "../../contexts/FrontendApi";
import { ActivityChart, SwarmChart } from "./ActivityChart";
import Portlet, { PortletProps } from "./Portlet";

type Tab = "activity" | "swarms";

type DebuggerProps = Omit<PortletProps, "children">;

const Debugger: React.FC<DebuggerProps> = ({ onClose, isOpen }) => {
  const client = useClient();
  const [selectedTab, setSelectedTab] = useState<Tab>();

  const [, pprof] = useLazyCall("debug", "pProf", {
    onComplete: ({ data, name }) => {
      const a = document.createElement("a");
      a.href = URL.createObjectURL(new Blob([data], { type: "application/binary" }));
      a.download = `${name}-${new Date().toISOString()}.profile`;
      a.click();
      URL.revokeObjectURL(a.href);
    },
  });

  const handleReadMetricsClick = async () => {
    const { data } = await client.debug.readMetrics({
      format: MetricsFormat.METRICS_FORMAT_PROTO_DELIM,
    });

    const metricFamilies: MetricFamily[] = [];
    for (const r = new PBReader(data); r.pos < r.len; ) {
      metricFamilies.push(MetricFamily.decode(r, r.uint32()));
    }
    console.log(metricFamilies);
  };

  let content: React.ReactNode = null;
  switch (selectedTab) {
    case "activity":
      content = <ActivityChart />;
      break;
    case "swarms":
      content = <SwarmChart />;
      break;
  }

  return (
    <Portlet onClose={onClose} isOpen={isOpen}>
      <div className="debugger__tabs">
        <button className="debugger__tabs__tab" onClick={() => setSelectedTab("activity")}>
          activity
        </button>
        <button className="debugger__tabs__tab" onClick={() => setSelectedTab("swarms")}>
          swarms
        </button>
        <button className="debugger__tabs__tab" onClick={() => pprof({ name: "allocs" })}>
          allocs <FiDownload />
        </button>
        <button className="debugger__tabs__tab" onClick={() => pprof({ name: "goroutine" })}>
          goroutine <FiDownload />
        </button>
        <button className="debugger__tabs__tab" onClick={() => pprof({ name: "heap" })}>
          heap <FiDownload />
        </button>
        <button className="debugger__tabs__tab" onClick={handleReadMetricsClick}>
          read metrics
        </button>
      </div>
      <div>{content}</div>
    </Portlet>
  );
};

export default Debugger;
