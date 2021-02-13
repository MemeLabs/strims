/* eslint-disable no-console */

import parsePrometheusTextFormat from "parse-prometheus-text-format";
import React from "react";
import { Sparklines, SparklinesBars } from "react-sparklines";

import { ReadMetricsRequest } from "../apis/strims/debug/v1/debug";
import { MainLayout } from "../components/MainLayout";
import { useClient, useLazyCall } from "../contexts/FrontendApi";
import { useProfile } from "../contexts/Profile";
import { useTheme } from "../contexts/Theme";

type PrometheusNumericMetricValue = {
  value: string;
  labels?: { [label: string]: string };
};

type PrometheusSummaryMetricValue = {
  labels?: { [label: string]: string };
  buckets?: { [label: string]: string };
  quantiles?: { [label: string]: string };
  count: string;
  sum: string;
};

type PrometheusNumericMetric = {
  name: string;
  help: string;
  type: "GAUGE" | "UNTYPED" | "COUNTER";
  metrics: PrometheusNumericMetricValue[];
};

type PrometheusSummaryMetric = {
  name: string;
  help: string;
  type: "SUMMARY" | "HISTOGRAM";
  metrics: PrometheusSummaryMetricValue[];
};

type PrometheusMetric =
  | ({ name: "go_gc_duration_seconds" } & PrometheusSummaryMetric)
  | ({ name: "go_goroutines" } & PrometheusNumericMetric)
  | ({ name: "go_info" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_alloc_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_alloc_bytes_total" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_buck_hash_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_frees" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_frees_total" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_gc_cpu_fraction" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_gc_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_heap_alloc_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_heap_idle_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_heap_inuse_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_heap_objects" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_heap_released_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_heap_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_last_gc_time_seconds" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_lookups" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_lookups_total" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_mallocs" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_mallocs_total" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_mcache_inuse_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_mcache_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_mspan_inuse_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_mspan_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_next_gc_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_other_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_stack_inuse_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_stack_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_memstats_sys_bytes" } & PrometheusNumericMetric)
  | ({ name: "go_threads" } & PrometheusNumericMetric)
  | ({ name: "strims_ppspp_channel" } & PrometheusNumericMetric)
  | ({ name: "strims_ppspp_scheduler_new_available_bins" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_dial_count" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_frame_hander_not_found_count" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_frame_handler_error_count" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_frame_read_bytes" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_frame_read_count" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_link_read_bytes" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_link_write_bytes" } & PrometheusNumericMetric)
  | ({ name: "strims_vpn_links_active" } & PrometheusNumericMetric);

type PrometheusNumericMetricSeries = {
  labels?: { [label: string]: string };
  values: number[];
  prev: number;
};

type PrometheusSummaryMetricSeries = {
  labels?: { [label: string]: string };
  values: {
    buckets?: { [label: string]: string };
    quantiles?: { [label: string]: string };
    count: string;
    sum: string;
  }[];
  prev: {
    buckets?: { [label: string]: string };
    quantiles?: { [label: string]: string };
    count: string;
    sum: string;
  };
};

type PrometheusNumericSeries = {
  name: string;
  help: string;
  metrics: PrometheusNumericMetricSeries[];
};

type PrometheusSummarySeries = {
  name: string;
  help: string;
  metrics: PrometheusSummaryMetricSeries[];
};

type Stats = {
  go_goroutines: PrometheusNumericSeries;
  go_info: PrometheusNumericSeries;
  go_memstats_alloc_bytes: PrometheusNumericSeries;
  go_memstats_alloc_bytes_total: PrometheusNumericSeries;
  go_memstats_buck_hash_sys_bytes: PrometheusNumericSeries;
  go_memstats_frees: PrometheusNumericSeries;
  go_memstats_frees_total: PrometheusNumericSeries;
  go_memstats_gc_cpu_fraction: PrometheusNumericSeries;
  go_memstats_gc_sys_bytes: PrometheusNumericSeries;
  go_memstats_heap_alloc_bytes: PrometheusNumericSeries;
  go_memstats_heap_idle_bytes: PrometheusNumericSeries;
  go_memstats_heap_inuse_bytes: PrometheusNumericSeries;
  go_memstats_heap_objects: PrometheusNumericSeries;
  go_memstats_heap_released_bytes: PrometheusNumericSeries;
  go_memstats_heap_sys_bytes: PrometheusNumericSeries;
  go_memstats_last_gc_time_seconds: PrometheusNumericSeries;
  go_memstats_lookups: PrometheusNumericSeries;
  go_memstats_lookups_total: PrometheusNumericSeries;
  go_memstats_mallocs: PrometheusNumericSeries;
  go_memstats_mallocs_total: PrometheusNumericSeries;
  go_memstats_mcache_inuse_bytes: PrometheusNumericSeries;
  go_memstats_mcache_sys_bytes: PrometheusNumericSeries;
  go_memstats_mspan_inuse_bytes: PrometheusNumericSeries;
  go_memstats_mspan_sys_bytes: PrometheusNumericSeries;
  go_memstats_next_gc_bytes: PrometheusNumericSeries;
  go_memstats_other_sys_bytes: PrometheusNumericSeries;
  go_memstats_stack_inuse_bytes: PrometheusNumericSeries;
  go_memstats_stack_sys_bytes: PrometheusNumericSeries;
  go_memstats_sys_bytes: PrometheusNumericSeries;
};

const numericMetricReducer = (prev: PrometheusNumericSeries, value: PrometheusNumericMetric) => {
  const prevMetrics = {} as { [labels: string]: PrometheusNumericMetricSeries };
  prev.metrics.forEach((m) => (prevMetrics[JSON.stringify(m.labels)] = m));

  return {
    ...prev,
    metrics: value.metrics.map((m) => ({
      labels: m.labels,
      values: [...(prevMetrics[JSON.stringify(m.labels)]?.values || []), parseFloat(m.value)],
      prev: parseFloat(m.value),
    })),
  };
};

const counterMetricReducer = (prev: PrometheusNumericSeries, value: PrometheusNumericMetric) => {
  const prevMetrics = {} as { [labels: string]: PrometheusNumericMetricSeries };
  prev.metrics.forEach((m) => (prevMetrics[JSON.stringify(m.labels)] = m));

  return {
    ...prev,
    metrics: value.metrics.map((m) => {
      const nextValue = parseFloat(m.value);
      const prevMetric = prevMetrics[JSON.stringify(m.labels)];
      const prevValue = prevMetric?.prev || 0;

      return {
        labels: m.labels,
        values: [...(prevMetric?.values || []), nextValue - prevValue],
        prev: nextValue,
      };
    }),
  };
};

const statsReducer = (stats: Stats, metrics: PrometheusMetric[]): Stats => {
  return metrics.reduce((stats: Stats, metric: PrometheusMetric): Stats => {
    switch (metric.name) {
      case "go_goroutines":
        return {
          ...stats,
          go_goroutines: numericMetricReducer(stats.go_goroutines, metric),
        };
      case "go_info":
        return {
          ...stats,
          go_info: numericMetricReducer(stats.go_info, metric),
        };
      case "go_memstats_alloc_bytes":
        return {
          ...stats,
          go_memstats_alloc_bytes: numericMetricReducer(stats.go_memstats_alloc_bytes, metric),
        };
      case "go_memstats_alloc_bytes_total":
        return {
          ...stats,
          go_memstats_alloc_bytes_total: counterMetricReducer(
            stats.go_memstats_alloc_bytes_total,
            metric
          ),
        };
      case "go_memstats_buck_hash_sys_bytes":
        return {
          ...stats,
          go_memstats_buck_hash_sys_bytes: numericMetricReducer(
            stats.go_memstats_buck_hash_sys_bytes,
            metric
          ),
        };
      case "go_memstats_frees":
        return {
          ...stats,
          go_memstats_frees: numericMetricReducer(stats.go_memstats_frees, metric),
        };
      case "go_memstats_frees_total":
        return {
          ...stats,
          go_memstats_frees_total: counterMetricReducer(stats.go_memstats_frees_total, metric),
        };
      case "go_memstats_gc_cpu_fraction":
        return {
          ...stats,
          go_memstats_gc_cpu_fraction: numericMetricReducer(
            stats.go_memstats_gc_cpu_fraction,
            metric
          ),
        };
      case "go_memstats_gc_sys_bytes":
        return {
          ...stats,
          go_memstats_gc_sys_bytes: numericMetricReducer(stats.go_memstats_gc_sys_bytes, metric),
        };
      case "go_memstats_heap_alloc_bytes":
        return {
          ...stats,
          go_memstats_heap_alloc_bytes: numericMetricReducer(
            stats.go_memstats_heap_alloc_bytes,
            metric
          ),
        };
      case "go_memstats_heap_idle_bytes":
        return {
          ...stats,
          go_memstats_heap_idle_bytes: numericMetricReducer(
            stats.go_memstats_heap_idle_bytes,
            metric
          ),
        };
      case "go_memstats_heap_inuse_bytes":
        return {
          ...stats,
          go_memstats_heap_inuse_bytes: numericMetricReducer(
            stats.go_memstats_heap_inuse_bytes,
            metric
          ),
        };
      case "go_memstats_heap_objects":
        return {
          ...stats,
          go_memstats_heap_objects: numericMetricReducer(stats.go_memstats_heap_objects, metric),
        };
      case "go_memstats_heap_released_bytes":
        return {
          ...stats,
          go_memstats_heap_released_bytes: numericMetricReducer(
            stats.go_memstats_heap_released_bytes,
            metric
          ),
        };
      case "go_memstats_heap_sys_bytes":
        return {
          ...stats,
          go_memstats_heap_sys_bytes: numericMetricReducer(
            stats.go_memstats_heap_sys_bytes,
            metric
          ),
        };
      case "go_memstats_last_gc_time_seconds":
        return {
          ...stats,
          go_memstats_last_gc_time_seconds: counterMetricReducer(
            stats.go_memstats_last_gc_time_seconds,
            metric
          ),
        };
      case "go_memstats_lookups":
        return {
          ...stats,
          go_memstats_lookups: numericMetricReducer(stats.go_memstats_lookups, metric),
        };
      case "go_memstats_lookups_total":
        return {
          ...stats,
          go_memstats_lookups_total: counterMetricReducer(stats.go_memstats_lookups_total, metric),
        };
      case "go_memstats_mallocs":
        return {
          ...stats,
          go_memstats_mallocs: numericMetricReducer(stats.go_memstats_mallocs, metric),
        };
      case "go_memstats_mallocs_total":
        return {
          ...stats,
          go_memstats_mallocs_total: counterMetricReducer(stats.go_memstats_mallocs_total, metric),
        };
      case "go_memstats_mcache_inuse_bytes":
        return {
          ...stats,
          go_memstats_mcache_inuse_bytes: numericMetricReducer(
            stats.go_memstats_mcache_inuse_bytes,
            metric
          ),
        };
      case "go_memstats_mcache_sys_bytes":
        return {
          ...stats,
          go_memstats_mcache_sys_bytes: numericMetricReducer(
            stats.go_memstats_mcache_sys_bytes,
            metric
          ),
        };
      case "go_memstats_mspan_inuse_bytes":
        return {
          ...stats,
          go_memstats_mspan_inuse_bytes: numericMetricReducer(
            stats.go_memstats_mspan_inuse_bytes,
            metric
          ),
        };
      case "go_memstats_mspan_sys_bytes":
        return {
          ...stats,
          go_memstats_mspan_sys_bytes: numericMetricReducer(
            stats.go_memstats_mspan_sys_bytes,
            metric
          ),
        };
      case "go_memstats_next_gc_bytes":
        return {
          ...stats,
          go_memstats_next_gc_bytes: numericMetricReducer(stats.go_memstats_next_gc_bytes, metric),
        };
      case "go_memstats_other_sys_bytes":
        return {
          ...stats,
          go_memstats_other_sys_bytes: numericMetricReducer(
            stats.go_memstats_other_sys_bytes,
            metric
          ),
        };
      case "go_memstats_stack_inuse_bytes":
        return {
          ...stats,
          go_memstats_stack_inuse_bytes: numericMetricReducer(
            stats.go_memstats_stack_inuse_bytes,
            metric
          ),
        };
      case "go_memstats_stack_sys_bytes":
        return {
          ...stats,
          go_memstats_stack_sys_bytes: numericMetricReducer(
            stats.go_memstats_stack_sys_bytes,
            metric
          ),
        };
      case "go_memstats_sys_bytes":
        return {
          ...stats,
          go_memstats_sys_bytes: numericMetricReducer(stats.go_memstats_sys_bytes, metric),
        };
    }
    return stats;
  }, stats);
};

const createNumericSeries = (name: string) => ({
  name,
  help: "",
  metrics: [],
});

const statsDefault: Stats = {
  go_goroutines: createNumericSeries("go_goroutines"),
  go_info: createNumericSeries("go_info"),
  go_memstats_alloc_bytes: createNumericSeries("go_memstats_alloc_bytes"),
  go_memstats_alloc_bytes_total: createNumericSeries("go_memstats_alloc_bytes_total"),
  go_memstats_buck_hash_sys_bytes: createNumericSeries("go_memstats_buck_hash_sys_bytes"),
  go_memstats_frees: createNumericSeries("go_memstats_frees"),
  go_memstats_frees_total: createNumericSeries("go_memstats_frees_total"),
  go_memstats_gc_cpu_fraction: createNumericSeries("go_memstats_gc_cpu_fraction"),
  go_memstats_gc_sys_bytes: createNumericSeries("go_memstats_gc_sys_bytes"),
  go_memstats_heap_alloc_bytes: createNumericSeries("go_memstats_heap_alloc_bytes"),
  go_memstats_heap_idle_bytes: createNumericSeries("go_memstats_heap_idle_bytes"),
  go_memstats_heap_inuse_bytes: createNumericSeries("go_memstats_heap_inuse_bytes"),
  go_memstats_heap_objects: createNumericSeries("go_memstats_heap_objects"),
  go_memstats_heap_released_bytes: createNumericSeries("go_memstats_heap_released_bytes"),
  go_memstats_heap_sys_bytes: createNumericSeries("go_memstats_heap_sys_bytes"),
  go_memstats_last_gc_time_seconds: createNumericSeries("go_memstats_last_gc_time_seconds"),
  go_memstats_lookups: createNumericSeries("go_memstats_lookups"),
  go_memstats_lookups_total: createNumericSeries("go_memstats_lookups_total"),
  go_memstats_mallocs: createNumericSeries("go_memstats_mallocs"),
  go_memstats_mallocs_total: createNumericSeries("go_memstats_mallocs_total"),
  go_memstats_mcache_inuse_bytes: createNumericSeries("go_memstats_mcache_inuse_bytes"),
  go_memstats_mcache_sys_bytes: createNumericSeries("go_memstats_mcache_sys_bytes"),
  go_memstats_mspan_inuse_bytes: createNumericSeries("go_memstats_mspan_inuse_bytes"),
  go_memstats_mspan_sys_bytes: createNumericSeries("go_memstats_mspan_sys_bytes"),
  go_memstats_next_gc_bytes: createNumericSeries("go_memstats_next_gc_bytes"),
  go_memstats_other_sys_bytes: createNumericSeries("go_memstats_other_sys_bytes"),
  go_memstats_stack_inuse_bytes: createNumericSeries("go_memstats_stack_inuse_bytes"),
  go_memstats_stack_sys_bytes: createNumericSeries("go_memstats_stack_sys_bytes"),
  go_memstats_sys_bytes: createNumericSeries("go_memstats_sys_bytes"),
};

const Directory = () => {
  const [{ colorScheme }, { setColorScheme }] = useTheme();
  const [{ profile }, { clearProfile }] = useProfile();

  const client = useClient();

  const [stats, reduceStats] = React.useReducer(statsReducer, statsDefault);

  React.useEffect(() => {
    const decoder = new TextDecoder();
    const ivl = setInterval(async () => {
      const res = await client.debug.readMetrics(
        new ReadMetricsRequest({
          format: 4,
        })
      );
      reduceStats(parsePrometheusTextFormat(decoder.decode(res.data)));
    }, 500);
    return () => clearInterval(ivl);
  }, []);

  return (
    <MainLayout>
      <main className="home_page__main">
        <section className="home_page__main__video">
          <table>
            <tbody>
              <tr>
                <td>go_goroutines</td>
                <td>{stats.go_goroutines.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_goroutines.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_info</td>
                <td>{stats.go_info.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_info.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_alloc_bytes</td>
                <td>{stats.go_memstats_alloc_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_alloc_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_alloc_bytes_total</td>
                <td>{stats.go_memstats_alloc_bytes_total.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_alloc_bytes_total.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_buck_hash_sys_bytes</td>
                <td>{stats.go_memstats_buck_hash_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_buck_hash_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_frees</td>
                <td>{stats.go_memstats_frees.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_frees.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_frees_total</td>
                <td>{stats.go_memstats_frees_total.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_frees_total.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_gc_cpu_fraction</td>
                <td>{stats.go_memstats_gc_cpu_fraction.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_gc_cpu_fraction.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_gc_sys_bytes</td>
                <td>{stats.go_memstats_gc_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_gc_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_heap_alloc_bytes</td>
                <td>{stats.go_memstats_heap_alloc_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_heap_alloc_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_heap_idle_bytes</td>
                <td>{stats.go_memstats_heap_idle_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_heap_idle_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_heap_inuse_bytes</td>
                <td>{stats.go_memstats_heap_inuse_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_heap_inuse_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_heap_objects</td>
                <td>{stats.go_memstats_heap_objects.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_heap_objects.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_heap_released_bytes</td>
                <td>{stats.go_memstats_heap_released_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_heap_released_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_heap_sys_bytes</td>
                <td>{stats.go_memstats_heap_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_heap_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_last_gc_time_seconds</td>
                <td>{stats.go_memstats_last_gc_time_seconds.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_last_gc_time_seconds.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_lookups</td>
                <td>{stats.go_memstats_lookups.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_lookups.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_lookups_total</td>
                <td>{stats.go_memstats_lookups_total.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_lookups_total.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_mallocs</td>
                <td>{stats.go_memstats_mallocs.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_mallocs.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_mallocs_total</td>
                <td>{stats.go_memstats_mallocs_total.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_mallocs_total.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_mcache_inuse_bytes</td>
                <td>{stats.go_memstats_mcache_inuse_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_mcache_inuse_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_mcache_sys_bytes</td>
                <td>{stats.go_memstats_mcache_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_mcache_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_mspan_inuse_bytes</td>
                <td>{stats.go_memstats_mspan_inuse_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_mspan_inuse_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_mspan_sys_bytes</td>
                <td>{stats.go_memstats_mspan_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_mspan_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_next_gc_bytes</td>
                <td>{stats.go_memstats_next_gc_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_next_gc_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_other_sys_bytes</td>
                <td>{stats.go_memstats_other_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_other_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_stack_inuse_bytes</td>
                <td>{stats.go_memstats_stack_inuse_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_stack_inuse_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_stack_sys_bytes</td>
                <td>{stats.go_memstats_stack_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_stack_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
              <tr>
                <td>go_memstats_sys_bytes</td>
                <td>{stats.go_memstats_sys_bytes.metrics[0]?.prev?.toLocaleString()}</td>
                <td style={{ height: "20px", width: "360px" }}>
                  <Sparklines
                    data={stats.go_memstats_sys_bytes.metrics[0]?.values || []}
                    limit={60}
                    width={360}
                    height={20}
                  >
                    <SparklinesBars />
                  </Sparklines>
                </td>
              </tr>
            </tbody>
          </table>
        </section>
      </main>
      <aside className="home_page__right">
        <header className="home_page__subheader"></header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat">chat</div>
      </aside>
    </MainLayout>
  );
};

export default Directory;
