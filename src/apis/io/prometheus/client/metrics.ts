import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Timestamp as google_protobuf_Timestamp,
  ITimestamp as google_protobuf_ITimestamp,
} from "../../../google/protobuf/timestamp";

export type ILabelPair = {
  name?: string;
  value?: string;
}

export class LabelPair {
  name: string;
  value: string;

  constructor(v?: ILabelPair) {
    this.name = v?.name || "";
    this.value = v?.value || "";
  }

  static encode(m: LabelPair, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name) w.uint32(10).string(m.name);
    if (m.value) w.uint32(18).string(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): LabelPair {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new LabelPair();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.value = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IGauge = {
  value?: number;
}

export class Gauge {
  value: number;

  constructor(v?: IGauge) {
    this.value = v?.value || 0;
  }

  static encode(m: Gauge, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(9).double(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Gauge {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Gauge();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.double();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ICounter = {
  value?: number;
  exemplar?: IExemplar | undefined;
}

export class Counter {
  value: number;
  exemplar: Exemplar | undefined;

  constructor(v?: ICounter) {
    this.value = v?.value || 0;
    this.exemplar = v?.exemplar && new Exemplar(v.exemplar);
  }

  static encode(m: Counter, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(9).double(m.value);
    if (m.exemplar) Exemplar.encode(m.exemplar, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Counter {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Counter();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.double();
        break;
        case 2:
        m.exemplar = Exemplar.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IQuantile = {
  quantile?: number;
  value?: number;
}

export class Quantile {
  quantile: number;
  value: number;

  constructor(v?: IQuantile) {
    this.quantile = v?.quantile || 0;
    this.value = v?.value || 0;
  }

  static encode(m: Quantile, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.quantile) w.uint32(9).double(m.quantile);
    if (m.value) w.uint32(17).double(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Quantile {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Quantile();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.quantile = r.double();
        break;
        case 2:
        m.value = r.double();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type ISummary = {
  sampleCount?: bigint;
  sampleSum?: number;
  quantile?: IQuantile[];
}

export class Summary {
  sampleCount: bigint;
  sampleSum: number;
  quantile: Quantile[];

  constructor(v?: ISummary) {
    this.sampleCount = v?.sampleCount || BigInt(0);
    this.sampleSum = v?.sampleSum || 0;
    this.quantile = v?.quantile ? v.quantile.map(v => new Quantile(v)) : [];
  }

  static encode(m: Summary, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.sampleCount) w.uint32(8).uint64(m.sampleCount);
    if (m.sampleSum) w.uint32(17).double(m.sampleSum);
    for (const v of m.quantile) Quantile.encode(v, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Summary {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Summary();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.sampleCount = r.uint64();
        break;
        case 2:
        m.sampleSum = r.double();
        break;
        case 3:
        m.quantile.push(Quantile.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IUntyped = {
  value?: number;
}

export class Untyped {
  value: number;

  constructor(v?: IUntyped) {
    this.value = v?.value || 0;
  }

  static encode(m: Untyped, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.value) w.uint32(9).double(m.value);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Untyped {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Untyped();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.value = r.double();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IHistogram = {
  sampleCount?: bigint;
  sampleSum?: number;
  bucket?: IBucket[];
}

export class Histogram {
  sampleCount: bigint;
  sampleSum: number;
  bucket: Bucket[];

  constructor(v?: IHistogram) {
    this.sampleCount = v?.sampleCount || BigInt(0);
    this.sampleSum = v?.sampleSum || 0;
    this.bucket = v?.bucket ? v.bucket.map(v => new Bucket(v)) : [];
  }

  static encode(m: Histogram, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.sampleCount) w.uint32(8).uint64(m.sampleCount);
    if (m.sampleSum) w.uint32(17).double(m.sampleSum);
    for (const v of m.bucket) Bucket.encode(v, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Histogram {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Histogram();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.sampleCount = r.uint64();
        break;
        case 2:
        m.sampleSum = r.double();
        break;
        case 3:
        m.bucket.push(Bucket.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IBucket = {
  cumulativeCount?: bigint;
  upperBound?: number;
  exemplar?: IExemplar | undefined;
}

export class Bucket {
  cumulativeCount: bigint;
  upperBound: number;
  exemplar: Exemplar | undefined;

  constructor(v?: IBucket) {
    this.cumulativeCount = v?.cumulativeCount || BigInt(0);
    this.upperBound = v?.upperBound || 0;
    this.exemplar = v?.exemplar && new Exemplar(v.exemplar);
  }

  static encode(m: Bucket, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.cumulativeCount) w.uint32(8).uint64(m.cumulativeCount);
    if (m.upperBound) w.uint32(17).double(m.upperBound);
    if (m.exemplar) Exemplar.encode(m.exemplar, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Bucket {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Bucket();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.cumulativeCount = r.uint64();
        break;
        case 2:
        m.upperBound = r.double();
        break;
        case 3:
        m.exemplar = Exemplar.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IExemplar = {
  label?: ILabelPair[];
  value?: number;
  timestamp?: google_protobuf_ITimestamp | undefined;
}

export class Exemplar {
  label: LabelPair[];
  value: number;
  timestamp: google_protobuf_Timestamp | undefined;

  constructor(v?: IExemplar) {
    this.label = v?.label ? v.label.map(v => new LabelPair(v)) : [];
    this.value = v?.value || 0;
    this.timestamp = v?.timestamp && new google_protobuf_Timestamp(v.timestamp);
  }

  static encode(m: Exemplar, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.label) LabelPair.encode(v, w.uint32(10).fork()).ldelim();
    if (m.value) w.uint32(17).double(m.value);
    if (m.timestamp) google_protobuf_Timestamp.encode(m.timestamp, w.uint32(26).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Exemplar {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Exemplar();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.label.push(LabelPair.decode(r, r.uint32()));
        break;
        case 2:
        m.value = r.double();
        break;
        case 3:
        m.timestamp = google_protobuf_Timestamp.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMetric = {
  label?: ILabelPair[];
  gauge?: IGauge | undefined;
  counter?: ICounter | undefined;
  summary?: ISummary | undefined;
  untyped?: IUntyped | undefined;
  histogram?: IHistogram | undefined;
  timestampMs?: bigint;
}

export class Metric {
  label: LabelPair[];
  gauge: Gauge | undefined;
  counter: Counter | undefined;
  summary: Summary | undefined;
  untyped: Untyped | undefined;
  histogram: Histogram | undefined;
  timestampMs: bigint;

  constructor(v?: IMetric) {
    this.label = v?.label ? v.label.map(v => new LabelPair(v)) : [];
    this.gauge = v?.gauge && new Gauge(v.gauge);
    this.counter = v?.counter && new Counter(v.counter);
    this.summary = v?.summary && new Summary(v.summary);
    this.untyped = v?.untyped && new Untyped(v.untyped);
    this.histogram = v?.histogram && new Histogram(v.histogram);
    this.timestampMs = v?.timestampMs || BigInt(0);
  }

  static encode(m: Metric, w?: Writer): Writer {
    if (!w) w = new Writer();
    for (const v of m.label) LabelPair.encode(v, w.uint32(10).fork()).ldelim();
    if (m.gauge) Gauge.encode(m.gauge, w.uint32(18).fork()).ldelim();
    if (m.counter) Counter.encode(m.counter, w.uint32(26).fork()).ldelim();
    if (m.summary) Summary.encode(m.summary, w.uint32(34).fork()).ldelim();
    if (m.untyped) Untyped.encode(m.untyped, w.uint32(42).fork()).ldelim();
    if (m.histogram) Histogram.encode(m.histogram, w.uint32(58).fork()).ldelim();
    if (m.timestampMs) w.uint32(48).int64(m.timestampMs);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Metric {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Metric();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.label.push(LabelPair.decode(r, r.uint32()));
        break;
        case 2:
        m.gauge = Gauge.decode(r, r.uint32());
        break;
        case 3:
        m.counter = Counter.decode(r, r.uint32());
        break;
        case 4:
        m.summary = Summary.decode(r, r.uint32());
        break;
        case 5:
        m.untyped = Untyped.decode(r, r.uint32());
        break;
        case 7:
        m.histogram = Histogram.decode(r, r.uint32());
        break;
        case 6:
        m.timestampMs = r.int64();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IMetricFamily = {
  name?: string;
  help?: string;
  type?: MetricType;
  metric?: IMetric[];
}

export class MetricFamily {
  name: string;
  help: string;
  type: MetricType;
  metric: Metric[];

  constructor(v?: IMetricFamily) {
    this.name = v?.name || "";
    this.help = v?.help || "";
    this.type = v?.type || 0;
    this.metric = v?.metric ? v.metric.map(v => new Metric(v)) : [];
  }

  static encode(m: MetricFamily, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.name) w.uint32(10).string(m.name);
    if (m.help) w.uint32(18).string(m.help);
    if (m.type) w.uint32(24).uint32(m.type);
    for (const v of m.metric) Metric.encode(v, w.uint32(34).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): MetricFamily {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new MetricFamily();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.name = r.string();
        break;
        case 2:
        m.help = r.string();
        break;
        case 3:
        m.type = r.uint32();
        break;
        case 4:
        m.metric.push(Metric.decode(r, r.uint32()));
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export enum MetricType {
  COUNTER = 0,
  GAUGE = 1,
  SUMMARY = 2,
  UNTYPED = 3,
  HISTOGRAM = 4,
}
