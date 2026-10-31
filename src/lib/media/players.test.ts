// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { StreamFormat, streamFormat } from "./mime";
import { PlayerKind, choosePlayer, preferredPlayer } from "./players";

const storage = (value: string | null) => ({ getItem: () => value });

describe("players", () => {
  it("a query parameter overrides the stored preference, and the baseline is the default", () => {
    assert.strictEqual(preferredPlayer("?player=lowlatency", storage("baseline")), "lowlatency");
    assert.strictEqual(preferredPlayer("?player=baseline", storage("lowlatency")), "baseline");
    assert.strictEqual(preferredPlayer("", storage("lowlatency")), "lowlatency");
    assert.strictEqual(preferredPlayer("?player=other", storage("nonsense")), "baseline");
    assert.strictEqual(preferredPlayer("", null), "baseline");
    const throwing = {
      getItem: (): string => {
        throw new Error("denied");
      },
    };
    assert.strictEqual(preferredPlayer("", throwing), "baseline");
  });

  it("unit-format streams play low-latency only when preferred and supported", () => {
    const units = { format: StreamFormat.Units };
    assert.strictEqual(
      choosePlayer({ ...units, preference: "lowlatency", lowLatency: true }),
      PlayerKind.LowLatency
    );
    assert.strictEqual(
      choosePlayer({ ...units, preference: "lowlatency", lowLatency: false }),
      PlayerKind.Units
    );
    assert.strictEqual(
      choosePlayer({ ...units, preference: "baseline", lowLatency: true }),
      PlayerKind.Units
    );
  });

  it("a stream of another format gets no player", () => {
    for (const mime of [
      'video/mp4;codecs="avc1.64001f,mp4a.40.2"',
      "video/webm;codecs=vp8",
      "application/octet-stream",
      "",
      undefined,
    ]) {
      for (const preference of ["baseline", "lowlatency"] as const) {
        assert.strictEqual(
          choosePlayer({ format: streamFormat(mime), preference, lowLatency: true }),
          PlayerKind.None,
          `${mime}`
        );
      }
    }
  });
});
