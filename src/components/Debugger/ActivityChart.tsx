// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import { format } from "d3-format";
import React, { useEffect, useState } from "react";
import {
  AreaSeries,
  HorizontalGridLines,
  LineSeries,
  LineSeriesPoint,
  XYPlot,
  YAxis,
} from "react-vis";

import { Metrics } from "../../apis/strims/debug/v1/metrics";
import { usePortletSize } from "./Portlet";
import {
  Point,
  Series,
  SwarmSeries,
  gauge,
  hex,
  latest,
  linkRates,
  rate,
  readerLatencies,
  swarmRates,
  timeDomain,
} from "./series";
import { HISTORY_SECONDS } from "./useMetricsHistory";

const PLOT_MARGIN_LARGE = {
  top: 5,
  left: 50,
  bottom: 5,
  right: 5,
};

const PLOT_MARGIN_SMALL = { ...PLOT_MARGIN_LARGE, left: 5 };

// Line colors in a fixed order; past the last they repeat, and the legend
// tells the lines apart.
const COLORS = [
  "#2a78d6",
  "#eb6834",
  "#1baf7a",
  "#eda100",
  "#e87ba4",
  "#008300",
  "#6250d6",
  "#e34948",
];

const lineColor = (i: number) => COLORS[i % COLORS.length];

const formatValue = format(".3~s");

const lastValue = (points: { y: number }[]) => {
  const last = points[points.length - 1];
  return last === undefined ? "-" : formatValue(last.y);
};

const seriesDomain = (v: { x: number }[], d: number = 240): [number, number] => {
  const max = v[v.length - 1]?.x ?? 0;
  return [max - d, max];
};

interface AbstractGraphProps {
  label: string;
  height: number;
  width: number;
  showAxes: boolean;
}

interface LineGraphProps extends AbstractGraphProps {
  values: LineSeriesPoint[][];
  // Names the lines in a legend with their latest values.
  labels?: string[];
  // The x range; by default each line's last 240 s.
  domain?: [number, number];
  className?: string;
}

const LineGraph: React.FC<LineGraphProps> = ({
  label,
  values,
  labels,
  domain,
  className,
  height,
  width,
  showAxes,
}) => (
  <div className={clsx("debugger_graph", className)}>
    <div className="debugger_graph__label">
      {label}
      {!labels && values.length === 1 && (
        <span className="debugger_graph__value">{lastValue(values[0])}</span>
      )}
    </div>
    <XYPlot
      className="debugger_graph__plot"
      width={width - 10}
      height={height}
      margin={showAxes ? PLOT_MARGIN_LARGE : PLOT_MARGIN_SMALL}
    >
      {showAxes && <HorizontalGridLines />}
      {showAxes && <YAxis tickFormat={format("~s")} />}
      {values.map((data, i) => (
        <LineSeries
          key={i}
          data={data}
          xDomain={domain ?? seriesDomain(data)}
          color={lineColor(i)}
          style={{ fill: "none" }}
        />
      ))}
    </XYPlot>
    {labels && (
      <div className="debugger_graph__legend">
        {labels.map((label, i) => (
          <span key={i} className="debugger_graph__legend__item">
            <i style={{ background: lineColor(i) }} />
            {label} <span className="debugger_graph__value">{lastValue(values[i] ?? [])}</span>
          </span>
        ))}
      </div>
    )}
  </div>
);

interface SeriesGraphProps extends AbstractGraphProps {
  series: Series[];
  domain: [number, number];
  className?: string;
}

// A line per series, with a legend.
const SeriesGraph: React.FC<SeriesGraphProps> = ({ series, ...props }) => (
  <LineGraph
    {...props}
    values={series.map(({ points }) => points)}
    labels={series.map(({ label }) => label)}
  />
);

interface ChartProps {
  history: Metrics[];
}

const useGraphSize = (largeHeight: number) => {
  const { width, height } = usePortletSize();
  const large = height > 600;
  return { width, height: large ? largeHeight : 20, showAxes: large };
};

export const ActivityChart: React.FC<ChartProps> = ({ history }) => {
  const size = useGraphSize(60);
  const domain = timeDomain(history, HISTORY_SECONDS);

  const live = gauge(history, (m) => m.memory?.liveBytes);
  const peak = gauge(history, (m) => m.memory?.peakBytes);
  const linear = gauge(history, (m) => m.memory?.linearMemoryBytes);
  const heap: Series[] = [
    { key: "live", label: "live", points: live },
    { key: "peak", label: "peak", points: peak },
  ];
  if (linear.some(({ y }) => y > 0)) {
    heap.push({ key: "linear", label: "linear memory", points: linear });
  }

  const graphs: [string, Point[]][] = [
    ["allocations/s", rate(history, (m) => m.memory?.allocations)],
    ["allocated B/s", rate(history, (m) => m.memory?.allocatedBytes)],
    ["loop iterations/s", rate(history, (m) => m.node?.driver?.loopIterations)],
    ["queries/s", rate(history, (m) => m.node?.driver?.queries)],
    ["query wakes/s", rate(history, (m) => m.node?.driver?.queryWakes)],
  ];

  return (
    <div>
      <SeriesGraph
        label={latest(history)?.memory ? "heap bytes" : "heap bytes (not counted)"}
        className="debugger_graph--memory"
        series={heap}
        domain={domain}
        {...size}
      />
      {graphs.map(([label, points]) => (
        <LineGraph key={label} label={label} values={[points]} domain={domain} {...size} />
      ))}
      <SeriesGraph
        label="link read B/s"
        series={linkRates(history, "read")}
        domain={domain}
        {...size}
      />
      <SeriesGraph
        label="link write B/s"
        series={linkRates(history, "write")}
        domain={domain}
        {...size}
      />
    </div>
  );
};

interface SwarmGraphProps extends AbstractGraphProps {
  values: SwarmSeries;
  domain: [number, number];
}

// Data in above the axis and out below it.
const SwarmGraph: React.FC<SwarmGraphProps> = ({
  label,
  values,
  domain,
  height,
  width,
  showAxes,
}) => (
  <div className="debugger_graph">
    <div className="debugger_graph__label">
      {label}
      <span className="debugger_graph__value">
        in {lastValue(values.in)}B/s out {lastValue(values.out)}B/s
      </span>
    </div>
    <XYPlot
      className="debugger_graph__plot"
      width={width - 10}
      height={height}
      margin={showAxes ? PLOT_MARGIN_LARGE : PLOT_MARGIN_SMALL}
    >
      {showAxes && <HorizontalGridLines />}
      {showAxes && <YAxis tickFormat={format("~s")} />}
      <AreaSeries data={values.in} xDomain={domain} />
      <AreaSeries data={values.out.map(({ x, y }) => ({ x, y: -y }))} xDomain={domain} />
      <LineSeries
        data={[
          { x: 0, y: 0 },
          { x: 1, y: 0 },
        ]}
        xDomain={[0, 1]}
        style={{ fill: "none" }}
      />
    </XYPlot>
  </div>
);

export const SwarmChart: React.FC<ChartProps> = ({ history }) => {
  const size = useGraphSize(80);
  const domain = timeDomain(history, HISTORY_SECONDS);

  return (
    <div>
      {swarmRates(history).map((values) => (
        <SwarmGraph
          key={values.key}
          label={`data ${values.label}`}
          values={values}
          domain={domain}
          {...size}
        />
      ))}
    </div>
  );
};

const formatMs = (ns?: bigint) => (ns === undefined ? "-" : `${(Number(ns) / 1e6).toFixed(0)} ms`);

export const MockStreamChart: React.FC<ChartProps> = ({ history }) => {
  const size = useGraphSize(80);
  const domain = timeDomain(history, HISTORY_SECONDS);
  const debug = latest(history)?.debug;

  return (
    <div>
      <SeriesGraph
        label="reader latency ms"
        series={readerLatencies(history)}
        domain={domain}
        {...size}
      />
      <table className="debugger_table">
        <caption>producers</caption>
        <thead>
          <tr>
            <th>id</th>
            <th>bitrate</th>
            <th>interval</th>
            <th>segments</th>
            <th>bytes</th>
          </tr>
        </thead>
        <tbody>
          {debug?.mockStreams.map((s) => (
            <tr key={s.id.toString()}>
              <td>{s.id.toString()}</td>
              <td>{s.bitrateKbps} kbps</td>
              <td>{s.segmentIntervalMs} ms</td>
              <td>{s.segments.toString()}</td>
              <td>{formatValue(Number(s.bytesWritten))}B</td>
            </tr>
          ))}
        </tbody>
      </table>
      <table className="debugger_table">
        <caption>readers</caption>
        <thead>
          <tr>
            <th>listing</th>
            <th>stream</th>
            <th>segments</th>
            <th>malformed</th>
            <th>discontinuities</th>
            <th>latency</th>
          </tr>
        </thead>
        <tbody>
          {debug?.readers.map((r) => (
            <tr key={`${hex(r.networkKey)}/${r.listingId.toString()}`}>
              <td>{r.listingId.toString()}</td>
              <td>{r.id?.value.toString() ?? "-"}</td>
              <td>{r.segments.toString()}</td>
              <td>{r.malformed.toString()}</td>
              <td>{r.discontinuities.toString()}</td>
              <td>{formatMs(r.latency?.value)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
};

interface RTCSeries {
  lastValue: { [id: string]: number };
  values: { [id: string]: LineSeriesPoint[] };
}

interface RTCChartValues {
  tx: RTCSeries;
  rx: RTCSeries;
}

const initRTCChartValues = {
  tx: { lastValue: {}, values: {} },
  rx: { lastValue: {}, values: {} },
};

const initRTCChartTotals = {
  timestamp: 0,
  bytesReceived: 0,
  bytesSent: 0,
  messagesReceived: 0,
  messagesSent: 0,
  openConns: 0,
};

export const RTCChart: React.FC = () => {
  const [values, setValues] = useState<RTCChartValues>(initRTCChartValues);
  const [total, setTotal] = useState(initRTCChartTotals);

  useEffect(() => {
    const sampleRate = 1000;
    const ivl = setInterval(async () => {
      const conns = (window as unknown as DebugWindow).__strims_rtc_peer_connections__ ?? [];

      const reports: Promise<RTCStatsReport>[] = [];
      for (const c of conns) {
        if (c.iceConnectionState === "connected") {
          reports.push(c.getStats(null));
        }
      }
      const results = await Promise.allSettled(reports);

      setValues((prev) => {
        const next = {
          tx: { lastValue: {}, values: {} },
          rx: { lastValue: {}, values: {} },
        };

        const updateNext = (typ: "tx" | "rx", id: string, ts: number, v: number) => {
          const lastValue = prev[typ].lastValue[id];
          next[typ].lastValue[id] = v;

          if (lastValue === undefined) {
            const refSeries = Object.values(prev[typ].values)[0] ?? [];
            const emptySeries = refSeries.map(({ x }) => ({ x, y: 0 }));
            emptySeries.push({ x: ts / sampleRate, y: 0 });

            next[typ].values[id] = emptySeries;
            return;
          }

          next[typ].values[id] = [
            ...prev[typ].values[id],
            {
              x: ts / sampleRate,
              y: v - lastValue,
            },
          ].slice(-240);
        };

        for (const res of results) {
          if (res.status === "fulfilled") {
            res.value.forEach((report) => {
              if (report.type === "data-channel" && report.label === "data") {
                updateNext("tx", report.id, report.timestamp, report.bytesSent);
                updateNext("rx", report.id, report.timestamp, report.bytesReceived);
              }
            });
          }
        }
        return next;
      });

      setTotal(() => {
        const next = {
          timestamp: 0,
          bytesReceived: 0,
          bytesSent: 0,
          messagesReceived: 0,
          messagesSent: 0,
          openConns: conns.length,
        };
        for (const res of results) {
          if (res.status === "fulfilled") {
            res.value.forEach((report) => {
              if (report.type === "data-channel" && report.label === "data") {
                next.timestamp = report.timestamp;
                next.bytesReceived += report.bytesReceived;
                next.bytesSent += report.bytesSent;
                next.messagesReceived += report.messagesReceived;
                next.messagesSent += report.messagesSent;
              }
            });
          }
        }
        return next;
      });
    }, sampleRate);
    return () => clearInterval(ivl);
  }, []);

  const { width, height } = usePortletSize();
  const large = height > 600;
  const graphHeight = large ? 80 : 20;

  return (
    <div>
      <LineGraph
        label="tx"
        values={Object.values(values.tx.values)}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <LineGraph
        label="rx"
        values={Object.values(values.rx.values)}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <table>
        <tbody>
          <tr>
            <td>timestamp</td>
            <td>{total.timestamp}</td>
          </tr>
          <tr>
            <td>bytesReceived</td>
            <td>{total.bytesReceived.toLocaleString()}</td>
          </tr>
          <tr>
            <td>bytesSent</td>
            <td>{total.bytesSent.toLocaleString()}</td>
          </tr>
          <tr>
            <td>messagesReceived</td>
            <td>{total.messagesReceived.toLocaleString()}</td>
          </tr>
          <tr>
            <td>messagesSent</td>
            <td>{total.messagesSent.toLocaleString()}</td>
          </tr>
          <tr>
            <td>openConns</td>
            <td>{total.openConns.toLocaleString()}</td>
          </tr>
        </tbody>
      </table>
    </div>
  );
};
