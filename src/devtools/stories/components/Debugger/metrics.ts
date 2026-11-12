// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { WatchMetricsResponse } from "../../../../apis/strims/debug/v1/debug";
import { UnimplementedDebugService } from "../../../../apis/strims/debug/v1/debug_rpc";
import { LinkKind, Metrics } from "../../../../apis/strims/debug/v1/metrics";
import { fixtures } from "../../../storybook";

// The Debugger stories' metrics: a node's snapshots a second apart, every
// value a function of the snapshot's index, so charts draw the same lines
// each time.

const SECOND = BigInt(1e9);

const big = (n: number): bigint => BigInt(Math.max(0, Math.round(n)));

// A wave around `base`, `amplitude` either side, `period` snapshots long.
const wave = (i: number, base: number, amplitude: number, period: number, phase = 0) =>
  base + amplitude * Math.sin((2 * Math.PI * (i + phase)) / period);

// A counter after `i` snapshots growing at a wavy rate.
const counter = (i: number, base: number, amplitude: number, period: number, phase = 0) => {
  let total = 0;
  for (let j = 0; j <= i; j++) {
    total += wave(j, base, amplitude, period, phase);
  }
  return big(total);
};

const links = [
  { host: "link:a", kind: LinkKind.LINK_KIND_WEBRTC, rate: 180_000 },
  { host: "link:b", kind: LinkKind.LINK_KIND_WEBRTC, rate: 95_000 },
  { host: "link:c", kind: LinkKind.LINK_KIND_WS, rate: 40_000 },
  { host: "link:d", kind: LinkKind.LINK_KIND_TCP, rate: 12_000 },
];

const swarms = [
  { swarm: BigInt(1), label: "comfy stream 1080p", rate: 600_000 },
  { swarm: BigInt(2), label: "", rate: 150_000 },
];

export interface MetricsOptions {
  /** How many snapshots (default 240, a full history). */
  count?: number;
  /** Leave out the memory metrics, as a node that doesn't count them does. */
  noMemory?: boolean;
}

// `count` snapshots, oldest first, the last at fixtureTime(0).
export const metricsHistory = ({ count = 240, noMemory = false }: MetricsOptions = {}): Metrics[] =>
  Array.from({ length: count }, (_, i) => {
    const time = fixtures.fixtureTime(0) * SECOND - BigInt(count - 1 - i) * SECOND;
    return new Metrics({
      time,
      memory: noMemory
        ? undefined
        : {
            liveBytes: big(wave(i, 48e6, 6e6, 40)),
            peakBytes: big(64e6 + i * 20e3),
            allocatedBytes: counter(i, 9e6, 3e6, 25),
            allocations: counter(i, 42e3, 12e3, 30, 5),
            deallocations: counter(i, 41e3, 12e3, 30, 6),
            reallocations: counter(i, 3e3, 1e3, 30),
            linearMemoryBytes: big(96e6),
          },
      node: {
        driver: {
          loopIterations: counter(i, 2400, 800, 20),
          queries: counter(i, 380, 120, 35, 10),
          queryWakes: counter(i, 120, 60, 15, 3),
          bytesRead: counter(i, 300e3, 80e3, 30),
          bytesWritten: counter(i, 250e3, 70e3, 30, 7),
        },
        links: links.map(({ host, kind, rate }, n) => ({
          hostId: fixtures.bytes(host, 32),
          kind,
          dialed: n % 2 === 0,
          mtu: BigInt(1200),
          metrics: {
            readBytes: counter(i, rate, rate / 3, 24 + n * 6, n * 4),
            readCount: counter(i, rate / 1000, rate / 3000, 24, n),
            writeBytes: counter(i, rate * 0.6, rate / 4, 30 + n * 4, n * 7),
            writeCount: counter(i, rate / 1400, rate / 4000, 30, n),
          },
        })),
        runner: {
          swarms: swarms.map(({ swarm, label }, n) => ({
            swarm,
            id: fixtures.bytes(`swarm:${swarm}`, 32),
            label,
            channels: BigInt(links.length),
            readers: BigInt(1),
            publishers: BigInt(n === 0 ? 0 : 1),
          })),
          peers: links.map(({ host }, p) => ({
            hostId: fixtures.bytes(host, 32),
            channels: swarms.map(({ swarm, rate }, s) => ({
              swarm,
              channel: BigInt(p * swarms.length + s + 1),
              peerChannel: BigInt(p * swarms.length + s + 101),
              metrics: {
                read: { count: counter(i, rate / (p + 1), rate / (3 * (p + 1)), 28, p * 3 + s) },
                write: { count: counter(i, rate / (2 * (p + 2)), rate / 8, 22, p + s * 5) },
              },
            })),
          })),
        },
        networks: [{ network: BigInt(1), links: BigInt(links.length), routedLinks: BigInt(2) }],
      },
      debug: {
        mockStreamsStarted: BigInt(2),
        segmentsWritten: big(i * 4),
        readersStarted: BigInt(2),
        segmentsRead: big(i * 4 - 3),
        mockStreams: [
          {
            id: BigInt(1),
            networkKey: fixtures.testNetworkKey,
            swarmUri: "magnet:?xt=urn:ppspp:mock-1",
            bitrateKbps: 4500,
            segmentIntervalMs: 500,
            timeoutMs: 30000,
            startedAt: fixtures.fixtureTime(-1),
            segments: big(i * 2),
            bytesWritten: big(i * 2 * 280e3),
            listingId: { value: BigInt(41) },
          },
          {
            id: BigInt(2),
            networkKey: fixtures.testNetworkKey,
            swarmUri: "magnet:?xt=urn:ppspp:mock-2",
            bitrateKbps: 1200,
            segmentIntervalMs: 1000,
            timeoutMs: 30000,
            startedAt: fixtures.fixtureTime(-1),
            segments: big(i),
            bytesWritten: big(i * 150e3),
          },
        ],
        readers: [
          {
            networkKey: fixtures.testNetworkKey,
            listingId: BigInt(41),
            swarmId: fixtures.bytes("swarm:1", 32),
            id: { value: BigInt(1) },
            segments: big(i * 2 - 1),
            malformed: BigInt(0),
            discontinuities: big(i / 40),
            latency: { value: big(wave(i, 850, 250, 30) * 1e6) },
            startedAt: fixtures.fixtureTime(-1),
          },
          {
            networkKey: fixtures.testNetworkKey,
            listingId: BigInt(42),
            swarmId: fixtures.bytes("swarm:2", 32),
            segments: big(i),
            malformed: BigInt(i > 60 ? 1 : 0),
            discontinuities: BigInt(0),
            latency: i % 17 === 0 ? undefined : { value: big(wave(i, 1600, 500, 45, 9) * 1e6) },
            startedAt: fixtures.fixtureTime(-1),
          },
        ],
      },
    });
  });

export type MetricsMode = "history" | "waiting" | "error";

// debug.watchMetrics: the history at once, then nothing more ("history");
// a stream that stays open and empty ("waiting"); or a refused call
// ("error"). The history is a minute's by default: the mock client's pipe
// passes a message a timer tick, so a longer one would still be arriving
// when the story settles.
export class DebugService extends UnimplementedDebugService {
  private streams: PassThrough[] = [];

  constructor(
    private mode: MetricsMode = "history",
    private history = metricsHistory({ count: 60 })
  ) {
    super();
  }

  watchMetrics(): PassThrough {
    if (this.mode === "error") {
      throw new Error("metrics are unavailable: the node is stopping");
    }
    const ch = new PassThrough({ objectMode: true });
    this.streams.push(ch);
    if (this.mode === "history") {
      for (const metrics of this.history) {
        ch.push(new WatchMetricsResponse({ metrics }));
      }
    }
    return ch;
  }

  destroy(): void {
    for (const ch of this.streams) {
      ch.destroy();
    }
  }
}
