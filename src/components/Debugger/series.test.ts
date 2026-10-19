// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  ILinkInfo,
  IMetrics,
  IRunnerPeerInfo,
  ISwarmInfo,
  LinkKind,
  Metrics,
} from "../../apis/strims/debug/v1/metrics";
import {
  gauge,
  jsonReplacer,
  linkRates,
  rate,
  readerLatencies,
  swarmRates,
  timeDomain,
} from "./series";

const T0 = 1700000000;

// A snapshot taken at T0 + s seconds.
const at = (s: number, m: Omit<IMetrics, "time"> = {}) =>
  new Metrics({ ...m, time: BigInt(T0 + s) * BigInt(1e9) });

const link = (hostId: number[], kind: LinkKind, readBytes: number): ILinkInfo => ({
  hostId: new Uint8Array(hostId),
  kind,
  metrics: { readBytes: BigInt(readBytes) },
});

const channel = (swarm: number, read: number, write: number) => ({
  swarm: BigInt(swarm),
  metrics: { read: { count: BigInt(read) }, write: { count: BigInt(write) } },
});

const runner = (swarms: ISwarmInfo[], peers: IRunnerPeerInfo[]) => ({
  node: { runner: { swarms, peers } },
});

describe("debugger series", () => {
  it("rates divide by the time between snapshots", () => {
    const history = [
      at(0, { memory: { allocations: BigInt(100) } }),
      at(1, { memory: { allocations: BigInt(150) } }),
      at(3, { memory: { allocations: BigInt(250) } }),
    ];
    assert.deepStrictEqual(
      rate(history, (m) => m.memory?.allocations),
      [
        { x: T0 + 1, y: 50 },
        { x: T0 + 3, y: 50 },
      ]
    );
  });

  it("rates skip snapshots without the value and clamp falls to 0", () => {
    const history = [
      at(0, { memory: { allocations: BigInt(100) } }),
      at(1),
      at(2, { memory: { allocations: BigInt(120) } }),
      at(3, { memory: { allocations: BigInt(20) } }),
      at(3, { memory: { allocations: BigInt(30) } }),
    ];
    assert.deepStrictEqual(
      rate(history, (m) => m.memory?.allocations),
      [{ x: T0 + 3, y: 0 }]
    );
  });

  it("gauges read each snapshot that has the value", () => {
    const history = [at(0), at(1, { memory: { liveBytes: BigInt(4096) } })];
    assert.deepStrictEqual(
      gauge(history, (m) => m.memory?.liveBytes),
      [{ x: T0 + 1, y: 4096 }]
    );
  });

  it("the domain ends at the latest snapshot", () => {
    assert.deepStrictEqual(timeDomain([at(0), at(10)], 240), [T0 - 230, T0 + 10]);
  });

  it("links get a series each and closed links stop theirs", () => {
    const a = [0xaa, 0xbb, 0xcc, 0xdd, 0xee];
    const b = [0x01, 0x02, 0x03, 0x04, 0x05];
    const history = [
      at(0, {
        node: { links: [link(a, LinkKind.LINK_KIND_WS, 0), link(b, LinkKind.LINK_KIND_TCP, 0)] },
      }),
      at(1, {
        node: { links: [link(a, LinkKind.LINK_KIND_WS, 10), link(b, LinkKind.LINK_KIND_TCP, 5)] },
      }),
      at(2, { node: { links: [link(a, LinkKind.LINK_KIND_WS, 30)] } }),
    ];
    assert.deepStrictEqual(linkRates(history, "read"), [
      {
        key: "aabbccddee/2",
        label: "aabbccdd ws",
        points: [
          { x: T0 + 1, y: 10 },
          { x: T0 + 2, y: 20 },
        ],
      },
      { key: "0102030405/1", label: "01020304 tcp", points: [{ x: T0 + 1, y: 5 }] },
    ]);
  });

  it("links to one host over one transport share a series", () => {
    const a = [0xaa];
    const history = [
      at(0, {
        node: {
          links: [link(a, LinkKind.LINK_KIND_WEBRTC, 1), link(a, LinkKind.LINK_KIND_WEBRTC, 2)],
        },
      }),
      at(1, {
        node: {
          links: [link(a, LinkKind.LINK_KIND_WEBRTC, 5), link(a, LinkKind.LINK_KIND_WEBRTC, 6)],
        },
      }),
      at(2, { node: { links: [link([], LinkKind.LINK_KIND_OTHER, 0)] } }),
    ];
    const series = linkRates(history, "read");
    assert.deepStrictEqual(series[0].points, [{ x: T0 + 1, y: 8 }]);
    assert.equal(series[0].label, "aa webrtc");
    assert.equal(series[1].label, "? other");
  });

  it("swarm rates sum channels by swarm key, labelled by the swarm", () => {
    const swarms: ISwarmInfo[] = [
      { swarm: BigInt(7), id: new Uint8Array([0x12, 0x34, 0x56, 0x78, 0x9a]), label: "" },
      { swarm: BigInt(9), id: new Uint8Array([0xff]), label: "video" },
    ];
    const history = [
      at(
        0,
        runner(swarms, [{ channels: [channel(7, 100, 10)] }, { channels: [channel(7, 0, 0)] }])
      ),
      at(
        2,
        runner(swarms, [{ channels: [channel(7, 300, 30)] }, { channels: [channel(7, 200, 50)] }])
      ),
      // The second peer's channel closes.
      at(3, runner(swarms, [{ channels: [channel(7, 310, 40)] }])),
    ];
    assert.deepStrictEqual(swarmRates(history), [
      {
        key: "7",
        label: "12345678",
        in: [
          { x: T0 + 2, y: 200 },
          { x: T0 + 3, y: 0 },
        ],
        out: [
          { x: T0 + 2, y: 35 },
          { x: T0 + 3, y: 0 },
        ],
      },
      {
        key: "9",
        label: "video",
        in: [
          { x: T0 + 2, y: 0 },
          { x: T0 + 3, y: 0 },
        ],
        out: [
          { x: T0 + 2, y: 0 },
          { x: T0 + 3, y: 0 },
        ],
      },
    ]);
  });

  it("reader latencies are milliseconds where a segment was read", () => {
    const reader = (latencyMs?: number) => ({
      networkKey: new Uint8Array([1]),
      listingId: BigInt(3),
      id: latencyMs === undefined ? undefined : { value: BigInt(42) },
      latency: latencyMs === undefined ? undefined : { value: BigInt(latencyMs * 1e6) },
    });
    const history = [
      at(0, { debug: { readers: [reader()] } }),
      at(1, { debug: { readers: [reader(250)] } }),
    ];
    assert.deepStrictEqual(readerLatencies(history), [
      { key: "01/3", label: "stream 42", points: [{ x: T0 + 1, y: 250 }] },
    ]);
  });

  it("the JSON replacer writes bigints as strings and bytes as hex", () => {
    const m = at(0, {
      node: { links: [link([0x0a, 0xff], LinkKind.LINK_KIND_TCP, 12)] },
      debug: {
        mockStreams: [{ id: BigInt(5), listingId: { value: BigInt("18446744073709551615") } }],
      },
    });
    const json = JSON.parse(JSON.stringify(m, jsonReplacer)) as {
      time: string;
      node: { links: { hostId: string; metrics: { readBytes: string } }[] };
      debug: { mockStreams: { id: string; listingId: { value: string } }[] };
    };
    assert.equal(json.time, (BigInt(T0) * BigInt(1e9)).toString());
    assert.equal(json.node.links[0].hostId, "0aff");
    assert.equal(json.node.links[0].metrics.readBytes, "12");
    assert.equal(json.debug.mockStreams[0].id, "5");
    assert.equal(json.debug.mockStreams[0].listingId.value, "18446744073709551615");
  });
});
