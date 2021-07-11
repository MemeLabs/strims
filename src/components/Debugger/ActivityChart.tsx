import PBReader from "@memelabs/protobuf/lib/pb/reader";
import React, { ReactElement } from "react";
import { useInterval } from "react-use";
import {
  AreaSeries,
  AreaSeriesPoint,
  HeatmapSeries,
  HeatmapSeriesPoint,
  LineSeries,
  LineSeriesPoint,
  XYPlot,
} from "react-vis";

import { LabelPair, Metric, MetricFamily } from "../../apis/io/prometheus/client/metrics";
import { MetricsFormat } from "../../apis/strims/debug/v1/debug";
import { useClient } from "../../contexts/FrontendApi";
import { usePortletSize } from "./Portlet";

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

interface AbstractGraphProps {
  label: string;
  height: number;
  width: number;
}

interface BarGraphProps extends AbstractGraphProps {
  values: LineSeriesPoint[][];
}

const BarGraph: React.FC<BarGraphProps> = ({ label, values, height, width }) => (
  <>
    <div style={{ "margin": "0 5px" }}>{label}</div>
    <XYPlot width={width - 10} height={height} margin={5}>
      {values.map((data, i) => (
        <LineSeries key={i} data={data} xDomain={[0, 240]} style={{ fill: "none" }} />
      ))}
    </XYPlot>
  </>
);

interface SummaryGraphProps extends AbstractGraphProps {
  values: SummarySeries;
}

const SummaryGraph: React.FC<SummaryGraphProps> = ({ label, values, height, width }) => (
  <>
    <div style={{ "margin": "0 5px" }}>{label}</div>
    <XYPlot width={width - 10} height={height} margin={5} axisDomain={[0, 240]}>
      <AreaSeries data={values.range} xDomain={[0, 240]} />
      <AreaSeries data={values.iqr} xDomain={[0, 240]} />
      <LineSeries data={values.median} xDomain={[0, 240]} style={{ fill: "none" }} />
    </XYPlot>
  </>
);

interface HeatmapGraphProps extends AbstractGraphProps {
  values: HeatmapSeriesPoint[];
}

const HeatmapGraph: React.FC<HeatmapGraphProps> = ({ label, values, height, width }) => (
  <>
    <div style={{ "margin": "0 5px" }}>{label}</div>
    <XYPlot width={width - 10} height={height} margin={5}>
      <HeatmapSeries data={values} xDomain={[0, 240]} />
    </XYPlot>
  </>
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
    graph = <BarGraph {...props} values={counterGraphValues(series)} />;
  } else if (series.metrics[0][0].gauge) {
    graph = <BarGraph {...props} values={gaugeGraphValues(series)} />;
  } else if (series.metrics[0][0].summary) {
    graph = <SummaryGraph {...props} values={summaryGraphValues(series)} />;
  } else if (series.metrics[0][0].histogram) {
    graph = <HeatmapGraph {...props} values={histogramGraphValues(series)} />;
  }

  return <div>{graph}</div>;
};

const ActivityChart: React.FC = () => {
  const client = useClient();
  const [metrics, dispatch] = React.useReducer(metricsReducer, {});

  useInterval(async () => {
    const { data } = await client.debug.readMetrics({
      format: MetricsFormat.METRICS_FORMAT_PROTO_DELIM,
    });

    const families: MetricFamily[] = [];
    for (const r = new PBReader(data); r.pos < r.len; ) {
      families.push(MetricFamily.decode(r, r.uint32()));
    }
    dispatch(families);
  }, 500);

  const { width, height } = usePortletSize();
  const graphHeight = height > 600 ? 60 : 20;

  return (
    <div style={{ height: "40px" }}>
      <Graph
        label="go_gc_duration_seconds"
        series={metrics["go_gc_duration_seconds"]}
        height={graphHeight}
        width={width}
      />
      <Graph
        label="go_goroutines"
        series={metrics["go_goroutines"]}
        height={graphHeight}
        width={width}
      />
      <Graph
        label="go_memstats_heap_objects"
        series={metrics["go_memstats_heap_objects"]}
        height={graphHeight}
        width={width}
      />
      <Graph
        label="go_memstats_alloc_bytes"
        series={metrics["go_memstats_alloc_bytes"]}
        height={graphHeight}
        width={width}
      />
      <Graph
        label="go_memstats_alloc_bytes_total"
        series={metrics["go_memstats_alloc_bytes_total"]}
        height={graphHeight}
        width={width}
      />
      <Graph
        label="go_memstats_mallocs_total"
        series={metrics["go_memstats_mallocs_total"]}
        height={graphHeight}
        width={width}
      />
      <Graph
        label="strims_vnic_link_read_bytes"
        series={metrics["strims_vnic_link_read_bytes"]}
        height={graphHeight}
        width={width}
      />
      <Graph
        label="strims_vnic_link_write_bytes"
        series={metrics["strims_vnic_link_write_bytes"]}
        height={graphHeight}
        width={width}
      />
    </div>
  );
};

export default ActivityChart;
