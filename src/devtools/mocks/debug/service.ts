// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

import * as debugv1 from "../../../apis/strims/debug/v1/debug";
import {
  DebugService as DebugServiceInterface,
  UnimplementedDebugService,
} from "../../../apis/strims/debug/v1/debug_rpc";
import { IMockStream, MockStream } from "../../../apis/strims/debug/v1/metrics";
import { bytes, fixtureTime } from "../../storybook/fixtures/ids";
import { networkKey } from "../../storybook/fixtures/networks";

const NS_PER_S = BigInt(1000000000);

// Mock stream `i`, from 1, on fixture network ((i - 1) % 5) + 1, started
// i * 7 minutes after the fixtures' epoch; the odd ones are listed.
export const mockStream = (i: number, opts: IMockStream = {}): MockStream => {
  const {
    bitrateKbps = [6000, 3000, 12000, 800][(i - 1) % 4],
    segmentIntervalMs = [1000, 500, 2000, 1000][(i - 1) % 4],
    timeoutMs = [0, 300000, 3600000, 90000][(i - 1) % 4],
    networkKey: key = networkKey(((i - 1) % 5) + 1),
    ...rest
  } = opts;
  const segments = BigInt(120 * i + 7);
  return new MockStream({
    id: BigInt(i),
    networkKey: key,
    swarmUri: `magnet:?xt=urn:ppspp:${Base64.fromUint8Array(bytes(`mock-swarm:${i}`, 20), true)}`,
    bitrateKbps,
    segmentIntervalMs,
    timeoutMs,
    startedAt: (fixtureTime() + BigInt(i * 7 * 60)) * NS_PER_S,
    segments,
    bytesWritten: (segments * BigInt(bitrateKbps) * BigInt(segmentIntervalMs)) / BigInt(8),
    listingId: i % 2 ? { value: BigInt(9000 + i) } : undefined,
    ...rest,
  });
};

export interface DebugServiceOptions {
  /** The stored config; default mock streams enabled, in fixture network 1. */
  config?: debugv1.IConfig;
  /** The running mock streams; default none. */
  mockStreams?: MockStream[];
}

// One node's debug settings and the mock streams it writes. The streams
// stand still (their segment counts don't grow), so polls render the same.
export default class DebugService
  extends UnimplementedDebugService
  implements DebugServiceInterface
{
  config: debugv1.Config;
  mockStreams: MockStream[];
  nextId: number;

  constructor({ config, mockStreams = [] }: DebugServiceOptions = {}) {
    super();
    this.config = new debugv1.Config(
      config ?? { enableMockStreams: true, mockStreamNetworkKey: networkKey(1) }
    );
    this.mockStreams = mockStreams;
    this.nextId = Math.max(0, ...mockStreams.map((s) => Number(s.id))) + 1;
  }

  getConfig(): debugv1.GetConfigResponse {
    return new debugv1.GetConfigResponse({ config: this.config });
  }

  setConfig(req: debugv1.SetConfigRequest): debugv1.SetConfigResponse {
    this.config = new debugv1.Config(req.config);
    return new debugv1.SetConfigResponse({ config: this.config });
  }

  startMockStream(req: debugv1.StartMockStreamRequest): debugv1.StartMockStreamResponse {
    const stream = mockStream(this.nextId++, { ...req, segments: BigInt(0), listingId: undefined });
    this.mockStreams = [...this.mockStreams, stream];
    return new debugv1.StartMockStreamResponse({ id: stream.id });
  }

  stopMockStream(req: debugv1.StopMockStreamRequest): debugv1.StopMockStreamResponse {
    this.mockStreams = this.mockStreams.filter((s) => s.id !== req.id);
    return new debugv1.StopMockStreamResponse();
  }

  listMockStreams(): debugv1.ListMockStreamsResponse {
    return new debugv1.ListMockStreamsResponse({ mockStreams: this.mockStreams });
  }
}
