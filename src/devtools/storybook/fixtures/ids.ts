// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

// A uint64 id as the generated messages carry it.
export const id = (n: number | bigint | string): bigint => BigInt(n);

// `count` ids from `start`.
export const ids = (count: number, start = 1): bigint[] =>
  Array.from({ length: count }, (_, i) => BigInt(start + i));

// `length` bytes that depend only on `seed` (xorshift32), for keys and serials.
export const bytes = (seed: number | string, length = 32): Uint8Array => {
  let x =
    typeof seed === "number"
      ? seed
      : Array.from(seed).reduce((h, c) => Math.imul(h ^ c.charCodeAt(0), 16777619), 2166136261);
  x = (x | 0) || 1;
  const out = new Uint8Array(length);
  for (let i = 0; i < length; i++) {
    x ^= x << 13;
    x ^= x >>> 17;
    x ^= x << 5;
    out[i] = x & 0xff;
  }
  return out;
};

// A 32 byte key for `seed`.
export const key = (seed: number | string): Uint8Array => bytes(`key:${seed}`, 32);

// Bytes as the app prints keys in URLs (URL-safe base64).
export const keyB64 = (b: Uint8Array): string => Base64.fromUint8Array(b, true);

/** Seconds since the epoch the fixtures date things from: 2026-01-01T00:00:00Z. */
export const FIXTURE_EPOCH = BigInt(1767225600);

// Seconds since the epoch, `days` after FIXTURE_EPOCH.
export const fixtureTime = (days = 0): bigint => FIXTURE_EPOCH + BigInt(Math.round(days * 86400));
