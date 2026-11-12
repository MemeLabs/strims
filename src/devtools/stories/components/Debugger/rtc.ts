// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect } from "react";

import { FIXTURE_EPOCH } from "../../../storybook/fixtures";

// The page's WebRTC peer connections as the RTC chart polls them
// (window.__strims_rtc_peer_connections__, which the wasm node sets): three
// connected ones whose data channels count up a sample per poll, and one
// still connecting, which the chart leaves out.

interface FakeConnection {
  iceConnectionState: RTCIceConnectionState;
  getStats: () => Promise<Map<string, unknown>>;
}

const START_MS = Number(FIXTURE_EPOCH) * 1000;

const connection = (n: number, rate: number): FakeConnection => {
  let polls = 0;
  return {
    iceConnectionState: "connected",
    getStats: () => {
      polls++;
      const id = `DC${n}`;
      return Promise.resolve(
        new Map([
          [
            id,
            {
              id,
              type: "data-channel",
              label: "data",
              timestamp: START_MS + polls * 1000,
              bytesSent: Math.round(polls * rate * (1 + 0.3 * Math.sin(polls / 2 + n))),
              bytesReceived: Math.round(polls * rate * 1.8 * (1 + 0.3 * Math.cos(polls / 3 + n))),
              messagesSent: polls * Math.round(rate / 900),
              messagesReceived: polls * Math.round(rate / 500),
            },
          ],
        ])
      );
    },
  };
};

// Sets the fake connections while mounted, putting back what was there.
export const useFakePeerConnections = (): void =>
  useEffect(() => {
    const w = window as unknown as DebugWindow;
    const prev = w.__strims_rtc_peer_connections__;
    w.__strims_rtc_peer_connections__ = [
      connection(1, 120_000),
      connection(2, 64_000),
      connection(3, 18_000),
      { iceConnectionState: "checking", getStats: () => Promise.resolve(new Map()) },
    ] as unknown as RTCPeerConnection[];
    return () => {
      w.__strims_rtc_peer_connections__ = prev;
    };
  }, []);
