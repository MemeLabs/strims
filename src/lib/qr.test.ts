// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import QRCode from "qrcode";

import { Pixels, decodeQR } from "./qr";

// `text`'s QR code as RGBA pixels: `scale` px a module, a four-module quiet
// zone, dark on light (a camera's picture is never pure black and white).
const render = (text: string, scale = 4): Pixels => {
  const { modules } = QRCode.create(text, { errorCorrectionLevel: "M" });
  const size = (modules.size + 8) * scale;
  const data = new Uint8ClampedArray(size * size * 4).fill(230);
  for (let r = 0; r < modules.size; r++) {
    for (let c = 0; c < modules.size; c++) {
      if (!modules.get(r, c)) {
        continue;
      }
      for (let y = 0; y < scale; y++) {
        for (let x = 0; x < scale; x++) {
          const i = (((r + 4) * scale + y) * size + (c + 4) * scale + x) * 4;
          data.set([20, 24, 28], i);
        }
      }
    }
  }
  for (let i = 3; i < data.length; i += 4) {
    data[i] = 255;
  }
  return { data, width: size, height: size };
};

describe("decodeQR", () => {
  it("reads a pairing token's QR code", () => {
    // The storybook fixture profile's, as a device shares it.
    const token =
      "EpkBCAESCXRlc3RfdXNlchogpKBFKa7MNoNQ0mNADKvTlLmXlnUlwLdIij24wmoKUQwiZggBEkBcPLYNDZx2BBxCNSpttvKAAEmVa/cTBiFVVHhHWVMTs0mRYWhFhmDA4QWyW//cTBKpQFQUF5ziL7ghm/A8HA76GiDVwW7GF/kcjl3pdCDKldd+ojwt25xaZ4m0elQgANHVpSgB";
    assert.equal(decodeQR(render(token)), token);
  });

  it("reads a code drawn small and one drawn large", () => {
    assert.equal(decodeQR(render("strims", 2)), "strims");
    assert.equal(decodeQR(render("strims", 9)), "strims");
  });

  it("reads a code in a larger picture", () => {
    const code = render("in the middle", 3);
    const width = code.width + 120;
    const height = code.height + 80;
    const data = new Uint8ClampedArray(width * height * 4).fill(200);
    for (let y = 0; y < code.height; y++) {
      data.set(
        code.data.subarray(y * code.width * 4, (y + 1) * code.width * 4),
        ((y + 40) * width + 60) * 4
      );
    }
    assert.equal(decodeQR({ data, width, height }), "in the middle");
  });

  it("is null where there's no code", () => {
    const width = 120;
    const height = 120;
    const data = new Uint8ClampedArray(width * height * 4);
    for (let i = 0; i < data.length; i++) {
      data[i] = (i * 7919) % 251;
    }
    assert.equal(decodeQR({ data, width, height }), null);
  });
});
