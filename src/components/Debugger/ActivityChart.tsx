// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import PBReader from "@memelabs/protobuf/lib/pb/reader";
import { format } from "d3-format";
import React, { ReactElement, useEffect, useState } from "react";
import {
  AreaSeries,
  AreaSeriesPoint,
  HeatmapSeries,
  HeatmapSeriesPoint,
  HorizontalGridLines,
  LineSeries,
  LineSeriesPoint,
  XYPlot,
  YAxis,
} from "react-vis";

import { Timestamp } from "../../apis/google/protobuf/timestamp";
import { Counter, LabelPair, Metric, MetricFamily } from "../../apis/io/prometheus/client/metrics";
import { MetricsFormat } from "../../apis/strims/debug/v1/debug";
import { useClient } from "../../contexts/FrontendApi";
import { usePortletSize } from "./Portlet";

const PLOT_MARGIN_LARGE = {
  top: 5,
  left: 50,
  bottom: 5,
  right: 5,
};

const PLOT_MARGIN_SMALL = { ...PLOT_MARGIN_LARGE, left: 5 };

interface MetricSeries {
  name: string;
  metrics: Metric[][];
}

type PrometheusMetrics = {
  [key: string]: MetricSeries;
};

const metricsReducer = (prev: PrometheusMetrics, families: MetricFamily[]) => {
  const next = { ...prev };
  families.forEach(({ name, metric }) => {
    const metrics = prev[name]?.metrics;
    const prune = metrics?.length == 241;

    next[name] = {
      name,
      metrics: [...(metrics?.slice(prune ? 1 : 0, 241) || []), metric],
    };
  });
  return next;
};

const formatKey = (l: LabelPair[]) => {
  const parts: string[] = [];
  l.forEach((p) => {
    parts.push(p.name);
    parts.push(p.value);
  });
  return parts.join("_");
};

const counterGraphValues = (series: MetricSeries) => {
  const values: {
    [key: string]: {
      prev: number;
      series: LineSeriesPoint[];
    };
  } = {};

  series.metrics.forEach((ms, i) => {
    ms.forEach((m) => {
      const key = formatKey(m.label);
      if (!(key in values)) {
        values[key] = {
          prev: m.counter.value,
          series: [],
        };
      } else {
        const v = values[key];
        v.series.push({
          x: i,
          y: m.counter.value - v.prev,
        });
        v.prev = m.counter.value;
      }
    });
  });

  return Object.values(values).map(({ series: values }) => values);
};

const gaugeGraphValues = (series: MetricSeries) => {
  const values: { [key: string]: LineSeriesPoint[] } = {};

  series.metrics.forEach((ms, i) => {
    ms.forEach((m) => {
      const key = formatKey(m.label);
      if (!(key in values)) {
        values[key] = [];
      } else {
        values[key].push({
          x: i,
          y: m.gauge.value,
        });
      }
    });
  });

  return Object.values(values);
};

interface SummarySeries {
  range: AreaSeriesPoint[];
  iqr: AreaSeriesPoint[];
  median: LineSeriesPoint[];
}

const summaryGraphValues = (series: MetricSeries): SummarySeries => ({
  range: series.metrics.slice(1).map((m, i) => ({
    x: i,
    y: m[0].summary.quantile[0].value,
    y0: m[0].summary.quantile[4].value,
  })),
  iqr: series.metrics.slice(1).map((m, i) => ({
    x: i,
    y: m[0].summary.quantile[1].value,
    y0: m[0].summary.quantile[3].value,
  })),
  median: series.metrics.slice(1).map((m, i) => ({
    x: i,
    y: m[0].summary.quantile[2].value,
  })),
});

const histogramGraphValues = (series: MetricSeries) => {
  const values: HeatmapSeriesPoint[] = [];
  series.metrics.slice(1).forEach((m, i) => {
    m[0].histogram.bucket.forEach((b) => {
      values.push({
        x: i,
        y: b.upperBound,
        color: Number(b.cumulativeCount),
      });
    });
  });
  return values;
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
}

const LineGraph: React.FC<LineGraphProps> = ({ label, values, height, width, showAxes }) => (
  <div className="debugger_graph">
    <div className="debugger_graph__label">{label}</div>
    <XYPlot
      className="debugger_graph__plot"
      width={width - 10}
      height={height}
      margin={showAxes ? PLOT_MARGIN_LARGE : PLOT_MARGIN_SMALL}
    >
      {showAxes && <HorizontalGridLines />}
      {showAxes && <YAxis tickFormat={format("~s")} />}
      {values.map((data, i) => (
        <LineSeries key={i} data={data} xDomain={seriesDomain(data)} style={{ fill: "none" }} />
      ))}
    </XYPlot>
  </div>
);

interface SummaryGraphProps extends AbstractGraphProps {
  values: SummarySeries;
}

const SummaryGraph: React.FC<SummaryGraphProps> = ({ label, values, height, width, showAxes }) => (
  <div className="debugger_graph">
    <div className="debugger_graph__label">{label}</div>
    <XYPlot
      className="debugger_graph__plot"
      width={width - 10}
      height={height}
      margin={showAxes ? PLOT_MARGIN_LARGE : PLOT_MARGIN_SMALL}
    >
      {showAxes && <HorizontalGridLines />}
      {showAxes && <YAxis tickFormat={format("~s")} />}
      <AreaSeries data={values.range} xDomain={seriesDomain(values.range)} />
      <AreaSeries data={values.iqr} xDomain={seriesDomain(values.iqr)} />
      <LineSeries
        data={values.median}
        xDomain={seriesDomain(values.median)}
        style={{ fill: "none" }}
      />
    </XYPlot>
  </div>
);

interface HeatmapGraphProps extends AbstractGraphProps {
  values: HeatmapSeriesPoint[];
}

const HeatmapGraph: React.FC<HeatmapGraphProps> = ({ label, values, height, width, showAxes }) => (
  <div className="debugger_graph">
    <div className="debugger_graph__label">{label}</div>
    <XYPlot
      className="debugger_graph__plot"
      width={width - 10}
      height={height}
      margin={showAxes ? PLOT_MARGIN_LARGE : PLOT_MARGIN_SMALL}
    >
      {showAxes && <HorizontalGridLines />}
      {showAxes && <YAxis tickFormat={format("~s")} />}
      <HeatmapSeries data={values} xDomain={seriesDomain(values)} />
    </XYPlot>
  </div>
);

interface GraphProps extends AbstractGraphProps {
  series?: MetricSeries;
}

const Graph: React.FC<GraphProps> = ({ series, ...props }) => {
  if (!series) {
    return null;
  }

  let graph: ReactElement;
  if (series.metrics[0][0].counter) {
    graph = <LineGraph {...props} values={counterGraphValues(series)} />;
  } else if (series.metrics[0][0].gauge) {
    graph = <LineGraph {...props} values={gaugeGraphValues(series)} />;
  } else if (series.metrics[0][0].summary) {
    graph = <SummaryGraph {...props} values={summaryGraphValues(series)} />;
  } else if (series.metrics[0][0].histogram) {
    graph = <HeatmapGraph {...props} values={histogramGraphValues(series)} />;
  }

  return <div>{graph}</div>;
};

const useMetrics = (dispatch: (families: MetricFamily[]) => void) => {
  const client = useClient();

  useEffect(() => {
    const events = client.debug.watchMetrics({
      format: MetricsFormat.METRICS_FORMAT_PROTO_DELIM,
      intervalMs: 500,
    });

    events.on("data", ({ data }) => {
      const families: MetricFamily[] = [];
      for (const r = new PBReader(data); r.pos < r.len; ) {
        families.push(MetricFamily.decode(r, r.uint32()));
      }
      dispatch(families);
    });

    return () => events.destroy();
  }, []);
};

export const ActivityChart: React.FC = () => {
  const [metrics, dispatch] = React.useReducer(metricsReducer, {});
  useMetrics(dispatch);

  const { width, height } = usePortletSize();
  const large = height > 600;
  const graphHeight = large ? 60 : 20;

  return (
    <div style={{ height: "40px" }}>
      <Graph
        label="go_gc_duration_seconds"
        series={metrics["go_gc_duration_seconds"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <Graph
        label="go_goroutines"
        series={metrics["go_goroutines"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <Graph
        label="go_memstats_heap_objects"
        series={metrics["go_memstats_heap_objects"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <Graph
        label="go_memstats_alloc_bytes"
        series={metrics["go_memstats_alloc_bytes"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <Graph
        label="go_memstats_alloc_bytes_total"
        series={metrics["go_memstats_alloc_bytes_total"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <Graph
        label="go_memstats_mallocs_total"
        series={metrics["go_memstats_mallocs_total"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <Graph
        label="strims_vnic_link_read_bytes"
        series={metrics["strims_vnic_link_read_bytes"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
      <Graph
        label="strims_vnic_link_write_bytes"
        series={metrics["strims_vnic_link_write_bytes"]}
        height={graphHeight}
        width={width}
        showAxes={large}
      />
    </div>
  );
};

type PrometheusSarmMetrics = {
  tick: number;
  swarm: {
    // label
    [key: string]: {
      // metric name
      [key: string]: {
        // direction
        [key: string]: {
          name: string;
          metrics: Counter[];
          tick: number;
        };
      };
    };
  };
  peer: {
    // label
    [key: string]: {
      // peerId
      [key: string]: {
        // metric name
        [key: string]: {
          // direction
          [key: string]: {
            name: string;
            metrics: Counter[];
          };
        };
      };
    };
  };
};

type SwarmMetricSeries = {
  [key: string]: {
    name: string;
    metrics: Counter[];
  };
};

const swarmGraphValues = (series: SwarmMetricSeries) => {
  const toSeries = (series: Counter[], d: number) => {
    let prev = series[0].value;
    return series.map(({ value }, x) => {
      const y = (value - prev) * d;
      prev = value;
      return { x, y };
    });
  };

  return {
    in: toSeries(series.in.metrics, 1),
    out: toSeries(series.out.metrics, -1),
  };
};

type SwarmSeries = {
  in: LineSeriesPoint[];
  out: LineSeriesPoint[];
};

interface SwarmGraphProps extends AbstractGraphProps {
  values: SwarmSeries;
}

const SwarmGraph: React.FC<SwarmGraphProps> = ({ label, values, height, width, showAxes }) => (
  <div className="debugger_graph">
    <div className="debugger_graph__label">{label}</div>
    <XYPlot
      className="debugger_graph__plot"
      width={width - 10}
      height={height}
      margin={showAxes ? PLOT_MARGIN_LARGE : PLOT_MARGIN_SMALL}
    >
      {showAxes && <HorizontalGridLines />}
      {showAxes && <YAxis tickFormat={format("~s")} />}
      <AreaSeries data={values.in} xDomain={seriesDomain(values.in)} />
      <AreaSeries data={values.out} xDomain={seriesDomain(values.out)} />
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

type M<T> = { [key: string]: T };

const unpack = <T extends M<any>>(next: M<T>, prev: M<T>, k: string): T => {
  if (next[k] === undefined) {
    return (next[k] = {} as T);
  }
  if (next[k] !== prev?.[k]) {
    return next[k];
  }
  return (next[k] = { ...next[k] });
};

const swarmsReducer = (prev: PrometheusSarmMetrics, families: MetricFamily[]) => {
  const next = {
    tick: prev.tick + 1,
    swarm: { ...prev.swarm },
    peer: { ...prev.peer },
  };

  const family = families.find(({ name }) => name === "strims_ppspp_channel");
  if (family) {
    family.metric.forEach(({ label, counter }) => {
      const [
        { value: direction },
        { value: swarmLabel },
        { value: peerId },
        { value: swarmId },
        { value: name },
      ] = label;
      const key = swarmLabel || swarmId.substring(0, 8);

      const p0 = unpack(next.peer, prev.peer, key);
      const p1 = unpack(p0, prev.peer[key], peerId);
      const p2 = unpack(p1, prev.peer[key]?.[peerId], name);
      const p3 = unpack(p2, prev.peer[key]?.[peerId]?.[name], direction);

      p3.name = name;
      const prune = p3.metrics?.length == 241;
      p3.metrics = [...(p3.metrics?.slice(prune ? 1 : 0, 241) || []), counter];

      const s0 = unpack(next.swarm, prev.swarm, key);
      const s1 = unpack(s0, prev.swarm[key], name);
      const s2 = unpack(s1, prev.swarm[key]?.[name], direction);

      s2.name = name;
      if (s2.tick === next.tick) {
        s2.metrics[s2.metrics.length - 1].value += counter.value;
      } else {
        const prune = s2.metrics?.length == 241;
        s2.metrics = [...(s2.metrics?.slice(prune ? 1 : 0, 241) || []), counter];
      }
      s2.tick = next.tick;
    });
  }

  return next;
};

const initialSwarmMetrics = {
  tick: 0,
  swarm: {},
  peer: {},
};

export const SwarmChart: React.FC = () => {
  const [metrics, dispatch] = React.useReducer(swarmsReducer, initialSwarmMetrics);
  useMetrics(dispatch);

  const { width, height } = usePortletSize();
  const large = height > 600;
  const graphHeight = large ? 80 : 20;

  return (
    <div style={{ height: "40px" }}>
      {Object.entries(metrics.swarm).map(([label, metrics]) => (
        <SwarmGraph
          key={label}
          label={`data_bytes ${label}`}
          values={swarmGraphValues(metrics["data_bytes"])}
          height={graphHeight}
          width={width}
          showAxes={large}
        />
      ))}
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
