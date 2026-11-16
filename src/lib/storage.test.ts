// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { readItem, writeItem } from "./storage";

// A Storage over a Map, with the getItem and setItem the helpers use.
const mapStorage = () => {
  const map = new Map<string, string>();
  const storage = {
    getItem: (key: string) => map.get(key) ?? null,
    setItem: (key: string, value: string) => void map.set(key, value),
  } as Storage;
  return { map, storage: () => storage };
};

const throwing = () =>
  ({
    getItem: () => {
      throw new Error("denied");
    },
    setItem: () => {
      throw new Error("quota");
    },
  }) as unknown as Storage;

describe("storage", () => {
  it("writes and reads an item", () => {
    const { map, storage } = mapStorage();
    assert.strictEqual(readItem("theme", storage), null);
    writeItem("theme", "light", storage);
    assert.strictEqual(map.get("theme"), "light");
    assert.strictEqual(readItem("theme", storage), "light");
  });

  it("reads nothing, and writes nothing, without storage or where it throws", () => {
    for (const storage of [() => null, throwing]) {
      assert.strictEqual(readItem("theme", storage), null);
      assert.doesNotThrow(() => writeItem("theme", "light", storage));
    }
  });
});
