import React from "react";

import {
  CapConnLoadLogResponse,
  CapConnLog,
  CapConnWatchLogsResponse,
} from "../../apis/strims/devtools/v1/ppspp/capconn";
import Nav from "../components/Nav";
import { useClient } from "../contexts/DevToolsApi";

const reduceLogs = (logs: string[], event: CapConnWatchLogsResponse): string[] => {
  switch (event.op) {
    case CapConnWatchLogsResponse.Op.CREATE:
      return [...logs, event.name].sort();
    case CapConnWatchLogsResponse.Op.REMOVE:
      return logs.filter((n) => n !== event.name);
  }
};

const useLogs = () => {
  const client = useClient();
  const [logs, dispatch] = React.useReducer(reduceLogs, []);

  React.useEffect(() => {
    const events = client.ppsppCapConn.watchLogs();
    events.on("data", dispatch);
    return () => events.destroy();
  }, []);

  return logs;
};

const Files = ({ onSelect }: { onSelect: (string) => void }) => {
  const logs = useLogs();

  return (
    <ul className="network_list">
      {logs.map((name, i) => (
        <li key={i} onClick={() => onSelect(name)}>
          {name}
        </li>
      ))}
    </ul>
  );
};

const baseLength = (b: bigint): bigint => {
  const t = b + BigInt(1);
  return t & -t;
};

const bounds = (vs: bigint[]): [bigint, bigint] => {
  if (vs.length === 0) {
    return [BigInt(0), BigInt(0)];
  }

  let min = vs[0];
  let max = vs[0];
  for (let i = 1; i < vs.length; i++) {
    if (vs[i] < min) min = vs[i];
    if (vs[i] > max) max = vs[i];
  }
  return [min, max];
};

type Scale = (bigint) => number;

const scale = (src: [bigint, bigint], dst: [number, number]): Scale => {
  const srcLen = src[1] - src[0];
  const dstLen = BigInt(dst[1] - dst[0]);
  if (srcLen === BigInt(0) || dstLen === BigInt(0)) {
    return (_: bigint) => dst[0];
  }
  return (v: bigint) => Number(((v - src[0]) * dstLen) / srcLen) + dst[0];
};

interface SparklineProps {
  height: number;
  width: number;
  x: bigint[];
  y: bigint[];
  margin?: number;
  xScale?: Scale;
  yScale?: Scale;
}

const Sparkline: React.FC<SparklineProps> = ({
  height,
  width,
  x,
  y,
  margin = 10,
  xScale = scale(bounds(x), [margin, width - margin]),
  yScale = scale(bounds(y), [height - margin, margin]),
}) => {
  if (x.length === 0) {
    return null;
  }

  const draw = (el: HTMLCanvasElement) => {
    const ctx = el.getContext("2d");
    ctx.fillStyle = "black";
    ctx.strokeStyle = "black";
    ctx.lineWidth = 1;

    for (let i = 0; i < x.length; i++) {
      ctx.moveTo(xScale(x[i]), height);
      ctx.lineTo(xScale(x[i]), yScale(y[i]));

      // ctx.moveTo(xScale(x[i]), yScale(y[i]));
      // ctx.arc(xScale(x[i]), yScale(y[i]), 2, 0, 2 * Math.PI);
    }

    ctx.stroke();
    // ctx.fill();
  };
  return <canvas ref={draw} height={height} width={width} />;
};

const millisecond = BigInt(1000000);

const Timeline = ({ data }: { data: CapConnLoadLogResponse }) => {
  const plots = data.log.peerLogs
    .map(({ label, events }, i) => {
      const times: bigint[] = [];
      const values: bigint[] = [];
      let lastTS = BigInt(0);
      events.forEach((e) => {
        if (e.code == CapConnLog.PeerLog.Event.Code.EVENT_CODE_READ) {
          for (let i = 0; i < e.messageTypes.length; i++) {
            if (e.messageTypes[i] == CapConnLog.PeerLog.Event.MessageType.MESSAGE_TYPE_DATA) {
              const ts = e.timestamp / (millisecond * BigInt(50));
              if (ts === lastTS) {
                values[values.length - 1] += baseLength(e.messageAddresses[i]);
                // values[values.length - 1]++;
              } else {
                times.push(ts);
                // const v = values[values.length - 1] || BigInt(0);
                values.push(baseLength(e.messageAddresses[i]) / BigInt(2));
                // values.push(BigInt(1));
                lastTS = ts;
              }
            }
          }
        }
      });

      return {
        label,
        times,
        values,
        xBounds: bounds(times),
      };
    })
    .sort((a, b) => a.label.localeCompare(b.label));

  const xBounds = plots.reduce<bigint[]>((b, { xBounds }) => [...b, ...xBounds], []);
  const xScale = scale(bounds(xBounds.filter((v) => v !== BigInt(0))), [0, 3000]);

  return (
    <>
      {plots.map(({ label, times, values }, i) => (
        <div key={i}>
          <div>{label}</div>
          <Sparkline height={50} width={3000} x={times} y={values} xScale={xScale} />
        </div>
      ))}
    </>
  );
};

const Home: React.FC = () => {
  const client = useClient();
  const [data, setData] = React.useState<CapConnLoadLogResponse>();
  const handleFileSelect = async (name: string) => {
    const log = await client.ppsppCapConn.loadLog({ name });
    setData(log);
  };

  React.useEffect(() => {
    void handleFileSelect("log-1612821803.bin");
  }, []);

  return (
    <div>
      <Nav />
      {data && <Timeline data={data} />}
      <Files onSelect={handleFileSelect} />
    </div>
  );
};

export default Home;
