// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { StreamFormat, hasCodecs, mimeEssence, streamFormat } from "./mime";

describe("streamFormat", () => {
  it("reads a listing's MIME type by its essence", () => {
    assert.strictEqual(
      streamFormat('video/x.strims-units;codecs="avc1.64001f,mp4a.40.2"'),
      StreamFormat.Units
    );
    assert.strictEqual(streamFormat(" Video/X.Strims-Units "), StreamFormat.Units);
    assert.strictEqual(streamFormat('video/mp4;codecs="avc1.64001f"'), StreamFormat.Fmp4);
    assert.strictEqual(streamFormat("video/mp2t"), StreamFormat.Unknown);
    assert.strictEqual(streamFormat(undefined), StreamFormat.Unknown);
  });
});

describe("mimeEssence", () => {
  it("drops the parameters", () => {
    assert.strictEqual(mimeEssence('video/mp4; codecs="mp4a.40.2"'), "video/mp4");
  });
});

describe("hasCodecs", () => {
  it("finds the codecs parameter", () => {
    assert.ok(hasCodecs('video/mp4;codecs="avc1.64001f"'));
    assert.ok(hasCodecs("video/mp4; Codecs=mp4a.40.5,avc1.64001F"));
    assert.ok(!hasCodecs("video/mp4"));
    assert.ok(!hasCodecs("video/mp4;profiles=iso6"));
  });
});
