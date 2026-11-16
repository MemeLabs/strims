// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { DevicePreferenceStore, DevicePreferences } from "../lib/devicePreferences";
import { createLayoutActions, initialLayoutState } from "./Layout";

// A store that records what it is given.
const recordingStore = () => {
  const saved: Partial<DevicePreferences>[] = [];
  const store: DevicePreferenceStore = {
    load: () => ({ swapMainPanels: false }),
    save: (p) => void saved.push(p),
  };
  return { saved, store };
};

describe("Layout actions", () => {
  it("toggle as useToggle does: a boolean sets, anything else flips", () => {
    const { toggleShowChat } = createLayoutActions(recordingStore().store);
    const s = initialLayoutState;
    assert.strictEqual(toggleShowChat(s).showChat, false);
    assert.strictEqual(toggleShowChat(s, true).showChat, true);
    assert.strictEqual(toggleShowChat(s, false).showChat, false);
    // A click handler's event, as `onClick={toggle}` passes.
    assert.strictEqual(toggleShowChat(s, { type: "click" } as unknown as boolean).showChat, false);
  });

  it("stores the swap, and nothing else", () => {
    const { saved, store } = recordingStore();
    const actions = createLayoutActions(store);
    let s = actions.toggleSwapMainPanels(initialLayoutState);
    assert.strictEqual(s.swapMainPanels, true);
    s = actions.toggleSwapMainPanels(s, false);
    assert.strictEqual(s.swapMainPanels, false);
    s = actions.toggleTheaterMode(s);
    s = actions.toggleOverlayOpen(s, true);
    assert.deepStrictEqual(saved, [{ swapMainPanels: true }, { swapMainPanels: false }]);
  });

  it("sets the overlay from a value or an updater", () => {
    const { setOverlayState, toggleOverlayOpen } = createLayoutActions(recordingStore().store);
    let s = setOverlayState(initialLayoutState, { open: true, transitioning: true });
    assert.deepStrictEqual(s.overlayState, { open: true, transitioning: true });
    s = setOverlayState(s, ({ open }) => ({ open: !open, transitioning: false }));
    assert.deepStrictEqual(s.overlayState, { open: false, transitioning: false });
    s = toggleOverlayOpen(s, true);
    assert.deepStrictEqual(s.overlayState, { open: true, transitioning: false });
  });
});
