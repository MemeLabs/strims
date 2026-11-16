// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { createDevicePreferenceStore } from "./devicePreferences";

// A Storage over a Map, with getItem and setItem the store uses.
const mapStorage = (entries: [string, string][] = []) => {
  const map = new Map(entries);
  return {
    map,
    storage: {
      getItem: (key: string) => map.get(key) ?? null,
      setItem: (key: string, value: string) => void map.set(key, value),
    } as Storage,
  };
};

describe("devicePreferences", () => {
  it("loads the defaults with nothing stored, no storage, or unreadable JSON", () => {
    for (const storage of [
      mapStorage().storage,
      null,
      mapStorage([["devicePreferences", "{not json"]]).storage,
      mapStorage([["devicePreferences", '"a string"']]).storage,
    ]) {
      assert.deepStrictEqual(createDevicePreferenceStore(() => storage).load(), {
        swapMainPanels: false,
      });
    }
  });

  it("loads what was stored, true only when it is true", () => {
    const load = (json: string) =>
      createDevicePreferenceStore(() => mapStorage([["devicePreferences", json]]).storage).load();
    assert.strictEqual(load('{"swapMainPanels":true}').swapMainPanels, true);
    assert.strictEqual(load('{"swapMainPanels":"true"}').swapMainPanels, false);
  });

  it("saves over what is stored, keeping keys it doesn't name", () => {
    const { map, storage } = mapStorage([["devicePreferences", '{"other":1}']]);
    const store = createDevicePreferenceStore(() => storage);
    store.save({ swapMainPanels: true });
    assert.deepStrictEqual(JSON.parse(map.get("devicePreferences")), {
      other: 1,
      swapMainPanels: true,
    });
    assert.strictEqual(store.load().swapMainPanels, true);
  });

  it("drops a save where storage throws", () => {
    const store = createDevicePreferenceStore(() => ({
      getItem: () => null,
      setItem: () => {
        throw new Error("quota");
      },
    }) as unknown as Storage);
    assert.doesNotThrow(() => store.save({ swapMainPanels: true }));
  });
});
