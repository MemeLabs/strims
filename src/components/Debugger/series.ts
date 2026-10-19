// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { LinkInfo, LinkKind, Metrics, SwarmInfo } from "../../apis/strims/debug/v1/metrics";

// A point on a chart, x in Unix seconds.
export interface Point {
  x: number;
  y: number;
}

// One line of a chart that plots several.
export interface Series {
  key: string;
  label: string;
  points: Point[];
}

// A swarm's data rates: bytes per second in (read) and out (written).
export interface SwarmSeries {
  key: string;
  label: string;
  in: Point[];
  out: Point[];
}

type Value = bigint | number;

interface Sample {
  value: Value;
  label: string;
}

type Samples = Map<string, Sample>;

export const hex = (bytes: Uint8Array): string =>
  Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");

export const toSeconds = (m: Metrics): number => Number(m.time) / 1e9;

export const latest = (history: Metrics[]): Metrics | undefined => history[history.length - 1];

// The x range of a chart: the window of the given length that ends at the
// latest snapshot.
export const timeDomain = (history: Metrics[], seconds: number): [number, number] => {
  const end = history.length ? toSeconds(latest(history)) : 0;
  return [end - seconds, end];
};

// The change per second from a to b. Counters only grow, so a fall (counted
// things leaving a sum, or a restart) reads as 0.
const perSecond = (prev: Metrics, next: Metrics, a: Value, b: Value): number | undefined => {
  const dt = Number(next.time - prev.time) / 1e9;
  if (!(dt > 0)) {
    return undefined;
  }
  return Math.max(0, (Number(b) - Number(a)) / dt);
};

// The value at each snapshot that has one.
export const gauge = (history: Metrics[], value: (m: Metrics) => Value | undefined): Point[] => {
  const points: Point[] = [];
  for (const m of history) {
    const v = value(m);
    if (v !== undefined) {
      points.push({ x: toSeconds(m), y: Number(v) });
    }
  }
  return points;
};

// A counter's rate between each pair of consecutive snapshots that both have
// it, at the later one.
export const rate = (history: Metrics[], value: (m: Metrics) => Value | undefined): Point[] => {
  const points: Point[] = [];
  for (let i = 1; i < history.length; i++) {
    const a = value(history[i - 1]);
    const b = value(history[i]);
    if (a === undefined || b === undefined) {
      continue;
    }
    const y = perSecond(history[i - 1], history[i], a, b);
    if (y !== undefined) {
      points.push({ x: toSeconds(history[i]), y });
    }
  }
  return points;
};

// One series per key in first appearance order, each labelled from the latest
// snapshot that has its key. A key that leaves the snapshots stops its line.
const keyed = (
  history: Metrics[],
  samples: Samples[],
  point: (i: number, sample: Sample, prev?: Sample) => number | undefined
): Series[] => {
  const series = new Map<string, Series>();
  samples.forEach((next, i) => {
    for (const [key, sample] of next) {
      let s = series.get(key);
      if (!s) {
        s = { key, label: sample.label, points: [] };
        series.set(key, s);
      }
      s.label = sample.label;
      const y = point(i, sample, samples[i - 1]?.get(key));
      if (y !== undefined) {
        s.points.push({ x: toSeconds(history[i]), y });
      }
    }
  });
  return Array.from(series.values());
};

const keyedGauges = (history: Metrics[], samples: Samples[]): Series[] =>
  keyed(history, samples, (_, { value }) => Number(value));

const keyedRates = (history: Metrics[], samples: Samples[]): Series[] =>
  keyed(
    history,
    samples,
    (i, { value }, prev) => prev && perSecond(history[i - 1], history[i], prev.value, value)
  );

// Adds value to key's sample, so samples that share a key count once.
const addSample = (samples: Samples, key: string, label: string, value: number) => {
  const prev = samples.get(key);
  samples.set(key, { label, value: Number(prev?.value ?? 0) + value });
};

const linkKindName = (kind: LinkKind): string =>
  (LinkKind[kind] ?? "LINK_KIND_OTHER").replace("LINK_KIND_", "").toLowerCase();

// A link's peer host id (empty until the handshake completes) and transport.
export const linkLabel = ({ hostId, kind }: LinkInfo): string =>
  `${hex(hostId).substring(0, 8) || "?"} ${linkKindName(kind)}`;

// Each link's bytes per second in one direction. Links to one host over one
// transport share a series.
export const linkRates = (history: Metrics[], direction: "read" | "write"): Series[] =>
  keyedRates(
    history,
    history.map((m) => {
      const samples: Samples = new Map();
      for (const link of m.node?.links ?? []) {
        const bytes = direction === "read" ? link.metrics?.readBytes : link.metrics?.writeBytes;
        addSample(samples, `${hex(link.hostId)}/${link.kind}`, linkLabel(link), Number(bytes ?? 0));
      }
      return samples;
    })
  );

// A swarm's label, or the first 8 hex digits of its id.
export const swarmLabel = ({ label, id }: SwarmInfo): string => label || hex(id).substring(0, 8);

// The data bytes each swarm's channels read and wrote, summed over peers, by
// swarm key. Swarms without channels count 0.
const swarmTotals = (m: Metrics): { read: Samples; write: Samples } => {
  const runner = m.node?.runner;
  const labels = new Map<string, string>();
  const read: Samples = new Map();
  const write: Samples = new Map();
  for (const swarm of runner?.swarms ?? []) {
    const key = swarm.swarm.toString();
    labels.set(key, swarmLabel(swarm));
    addSample(read, key, labels.get(key), 0);
    addSample(write, key, labels.get(key), 0);
  }
  for (const peer of runner?.peers ?? []) {
    for (const { swarm, metrics } of peer.channels) {
      const key = swarm.toString();
      const label = labels.get(key) ?? `swarm ${key}`;
      addSample(read, key, label, Number(metrics?.read?.count ?? 0));
      addSample(write, key, label, Number(metrics?.write?.count ?? 0));
    }
  }
  return { read, write };
};

// Each swarm's data rates. A channel that closes takes its bytes out of the
// sum, which reads as 0 rather than a negative rate.
export const swarmRates = (history: Metrics[]): SwarmSeries[] => {
  const totals = history.map(swarmTotals);
  const read = keyedRates(
    history,
    totals.map(({ read }) => read)
  );
  const write = new Map(
    keyedRates(
      history,
      totals.map(({ write }) => write)
    ).map((s) => [s.key, s])
  );
  return read.map(({ key, label, points }) => ({
    key,
    label,
    in: points,
    out: write.get(key)?.points ?? [],
  }));
};

// Each mock stream reader's latency in milliseconds, at the snapshots that
// have one.
export const readerLatencies = (history: Metrics[]): Series[] =>
  keyedGauges(
    history,
    history.map((m) => {
      const samples: Samples = new Map();
      for (const r of m.debug?.readers ?? []) {
        if (r.latency) {
          samples.set(`${hex(r.networkKey)}/${r.listingId}`, {
            label: r.id ? `stream ${r.id.value}` : `listing ${r.listingId}`,
            value: Number(r.latency.value) / 1e6,
          });
        }
      }
      return samples;
    })
  );

// A JSON.stringify replacer for snapshots: bigints become decimal strings and
// bytes hex.
export const jsonReplacer = (_key: string, value: unknown): unknown => {
  if (typeof value === "bigint") {
    return value.toString();
  }
  if (value instanceof Uint8Array) {
    return hex(value);
  }
  return value;
};
