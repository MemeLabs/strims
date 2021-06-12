import "../../styles/debugger.scss";

import React from "react";

import { useClient, useLazyCall } from "../../contexts/FrontendApi";
import Portlet, { PortletProps } from "./Portlet";

type DebuggerProps = PortletProps;

interface PProfDownload {
  name: string;
  url: string;
}

const Debugger: React.FC<DebuggerProps> = ({ onClose, isOpen }) => {
  const client = useClient();

  const [pprofData, pprof] = useLazyCall("debug", "pProf");
  const [pprofDownloads, setPProfDownloads] = React.useState([] as PProfDownload[]);

  const addDownload = (name: string, data: Uint8Array) => {
    const f = new File([data], name, {
      type: "application/binary",
    });
    setPProfDownloads((prev) => [...prev, { name, url: URL.createObjectURL(f) }]);
  };

  React.useEffect(() => {
    if (!pprofData.value) {
      return;
    }
    const now = new Date();
    addDownload(`${pprofData.value.name}-${now.toISOString()}.profile`, pprofData.value.data);
  }, [pprofData.value]);

  const handleReadMetricsClick = async () => {
    const { data } = await client.debug.readMetrics({ format: 0 });
    console.log(new TextDecoder().decode(data));
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
        {pprofDownloads.map(({ name, url }, i) => (
          <div className="debugger__downloads" key={i}>
            <a href={url} download={name}>
              {name}
            </a>
          </div>
        ))}
      </div>
    </Portlet>
  );
};

export default Debugger;
