// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useForm } from "react-hook-form";
import { useWindowSize } from "react-use";

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
  yScale = scale(bounds(y), [height - margin - 2, margin]),
}) => {
  const canvas = React.useRef<HTMLCanvasElement>();

  React.useEffect(() => {
    if (!canvas.current || x.length === 0) {
      return;
    }

    const ctx = canvas.current.getContext("2d");

    ctx.clearRect(0, 0, width, height);

    ctx.beginPath();
    for (let i = 0; i < x.length; i++) {
      ctx.moveTo(xScale(x[i]), height - margin);
      ctx.lineTo(xScale(x[i]), yScale(y[i]));
    }
    ctx.closePath();

    ctx.strokeStyle = "black";
    ctx.lineWidth = 1;

    ctx.stroke();
  }, [canvas.current, height, width, x, y]);

  if (x.length === 0) {
    return null;
  }
  return <canvas ref={canvas} height={height} width={width} />;
};

const millisecond = BigInt(1000000);

interface TimelineProps {
  data: CapConnLoadLogResponse;
  sparklineWidth?: number;
  sparklineHeight?: number;
  sparklineMargin?: number;
}

interface TimelineFormData {
  eventCode: CapConnLog.PeerLog.Event.Code;
  messageType: CapConnLog.PeerLog.Event.MessageType;
}

const Timeline = ({
  data,
  sparklineWidth = 3000,
  sparklineHeight = 50,
  sparklineMargin = 10,
}: TimelineProps) => {
  const { register, watch } = useForm<TimelineFormData>({
    mode: "onChange",
    defaultValues: {
      eventCode: CapConnLog.PeerLog.Event.Code.EVENT_CODE_READ,
      messageType: CapConnLog.PeerLog.Event.MessageType.MESSAGE_TYPE_DATA,
    },
  });
  const { eventCode, messageType } = watch();

  const [plots, xBounds, yBounds] = React.useMemo(() => {
    const plots = data.log.peerLogs
      .map(({ label, events }, i) => {
        const times: bigint[] = [];
        const values: bigint[] = [];
        let lastTS = BigInt(0);
        events.forEach((e) => {
          if (e.code == eventCode) {
            for (let i = 0; i < e.messageTypes.length; i++) {
              if (e.messageTypes[i] == messageType) {
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
          yBounds: bounds(values),
        };
      })
      .sort((a, b) => a.label.localeCompare(b.label));

    const xBounds = plots.reduce<bigint[]>((b, { xBounds }) => [...b, ...xBounds], []);
    const yBounds = plots.reduce<bigint[]>((b, { yBounds }) => [...b, ...yBounds], []);

    return [plots, xBounds, yBounds];
  }, [data, eventCode, messageType]);

  const xScale = React.useMemo(() => {
    return scale(bounds(xBounds.filter((v) => v !== BigInt(0))), [
      sparklineMargin,
      sparklineWidth - sparklineMargin,
    ]);
  }, [xBounds, sparklineWidth]);

  const yScale = React.useMemo(() => {
    return scale(bounds(yBounds), [sparklineHeight - sparklineMargin, sparklineMargin]);
  }, [yBounds, sparklineHeight]);

  return (
    <div className="network_timeline">
      <form className="network_timeline__controls">
        <select {...register("eventCode")}>
          <option value={CapConnLog.PeerLog.Event.Code.EVENT_CODE_READ}>READ</option>
          <option value={CapConnLog.PeerLog.Event.Code.EVENT_CODE_FLUSH}>WRITE</option>
        </select>
        <select {...register("messageType")}>
          {Object.entries(CapConnLog.PeerLog.Event.MessageType)
            .filter(([, id]) => typeof id === "number")
            .map(([label, id]) => (
              <option key={id} value={id}>
                {label}
              </option>
            ))}
        </select>
      </form>
      <div className="network_timeline__list">
        {plots.map(({ label, times, values }, i) => (
          <div key={i}>
            <div>{label}</div>
            <Sparkline
              height={sparklineHeight}
              width={sparklineWidth}
              x={times}
              y={values}
              xScale={xScale}
              yScale={yScale}
              margin={sparklineMargin}
            />
          </div>
        ))}
      </div>
    </div>
  );
};

const Home: React.FC = () => {
  const { width: windowWidth } = useWindowSize();

  const client = useClient();
  const [data, setData] = React.useState<CapConnLoadLogResponse>();
  const handleFileSelect = async (name: string) => {
    const log = await client.ppsppCapConn.loadLog({ name }, { timeout: 30000 });
    setData(log);
  };

  React.useEffect(() => {
    void handleFileSelect("log-1618716710.bin");
  }, []);

  return (
    <div>
      <Nav />
      {data && <Timeline data={data} sparklineWidth={windowWidth - 100} />}
      <Files onSelect={handleFileSelect} />
    </div>
  );
};

export default Home;
