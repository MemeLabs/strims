import "../../styles/debugger.scss";

import PBReader from "@memelabs/protobuf/lib/pb/reader";
import React from "react";

import { MetricFamily } from "../../apis/io/prometheus/client/metrics";
import { MetricsFormat } from "../../apis/strims/debug/v1/debug";
import { useClient, useLazyCall } from "../../contexts/FrontendApi";
import ActivityChart from "./ActivityChart";
import Portlet, { PortletProps } from "./Portlet";

type DebuggerProps = PortletProps;

const Debugger: React.FC<DebuggerProps> = ({ onClose, isOpen }) => {
  const client = useClient();

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

  return (
    <Portlet onClose={onClose} isOpen={isOpen}>
      <div className="debugger__tabs">
        <button className="debugger__tabs__tab" onClick={() => pprof({ name: "allocs" })}>
          allocs profile
        </button>
        <button className="debugger__tabs__tab" onClick={() => pprof({ name: "goroutine" })}>
          goroutine profile
        </button>
        <button className="debugger__tabs__tab" onClick={() => pprof({ name: "heap" })}>
          heap profile
        </button>
        <button className="debugger__tabs__tab" onClick={handleReadMetricsClick}>
          read metrics
        </button>
      </div>
      <div>
        <ActivityChart />
      </div>
    </Portlet>
  );
};

export default Debugger;
