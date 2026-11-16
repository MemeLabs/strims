// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { DevicePreferenceStore, DevicePreferences } from "../lib/devicePreferences";
import {
  chatOnScreen,
  createLayoutActions,
  createPreferenceSaver,
  initialLayoutState,
} from "./Layout";

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
    const { toggleShowChat } = createLayoutActions();
    const s = initialLayoutState;
    assert.strictEqual(toggleShowChat(s).showChat, false);
    assert.strictEqual(toggleShowChat(s, true).showChat, true);
    assert.strictEqual(toggleShowChat(s, false).showChat, false);
    // A click handler's event, as `onClick={toggle}` passes.
    assert.strictEqual(toggleShowChat(s, { type: "click" } as unknown as boolean).showChat, false);
  });

  it("return the state they were given when a toggle changes nothing", () => {
    const actions = createLayoutActions();
    const s = initialLayoutState;
    // The layout's toggleFullScreenChat(false) on each navigation.
    assert.strictEqual(actions.toggleFullScreenChat(s, false), s);
    assert.strictEqual(actions.toggleShowChat(s, true), s);
    assert.strictEqual(actions.toggleSwapMainPanels(s, false), s);
    assert.notStrictEqual(actions.toggleFullScreenChat(s, true), s);
  });

  it("toggle the swap without storing it", () => {
    const actions = createLayoutActions();
    let s = actions.toggleSwapMainPanels(initialLayoutState);
    assert.strictEqual(s.swapMainPanels, true);
    s = actions.toggleSwapMainPanels(s, false);
    assert.strictEqual(s.swapMainPanels, false);
  });

  it("set the overlay from a value or an updater", () => {
    const { setOverlayState, toggleOverlayOpen } = createLayoutActions();
    let s = setOverlayState(initialLayoutState, { open: true, transitioning: true });
    assert.deepStrictEqual(s.overlayState, { open: true, transitioning: true });
    s = setOverlayState(s, ({ open }) => ({ open: !open, transitioning: false }));
    assert.deepStrictEqual(s.overlayState, { open: false, transitioning: false });
    s = toggleOverlayOpen(s, true);
    assert.deepStrictEqual(s.overlayState, { open: true, transitioning: false });
  });
});

describe("chatOnScreen", () => {
  const layout = (open: boolean, showChat: boolean, transitioning = false) => ({
    ...initialLayoutState,
    overlayState: { open, transitioning },
    showChat,
  });

  // chatOnScreen over a run of layouts, from the chat off screen.
  const run = (layouts: ReturnType<typeof layout>[], portable = true) => {
    let shown = false;
    return layouts.map((l) => (shown = chatOnScreen(shown, l, portable)));
  };

  it("on a portable device, follows the drawer, not the panel", () => {
    // The closed drawer's peek shows none of the chat, its Shell mounted.
    assert.deepStrictEqual(run([layout(false, true)]), [false]);
    assert.deepStrictEqual(run([layout(true, true)]), [true]);
  });

  it("on a portable device, changes only once the drawer settles", () => {
    // A drag reports the drawer open past 10 px and closed short of them.
    const drag = (opens: boolean[]) => opens.map((open) => layout(open, true, true));
    // Dragged open, back and forth: shown once it settles open.
    assert.deepStrictEqual(run([...drag([true, false, true]), layout(true, true)]), [
      false,
      false,
      false,
      true,
    ]);
    // The open drawer grabbed (a tap's few px, a wobble) and let go: shown
    // throughout.
    const open = layout(true, true);
    assert.deepStrictEqual(run([open, ...drag([true, false, true]), open]), [
      true,
      true,
      true,
      true,
      true,
    ]);
    // Dragged closed: shown until it settles closed.
    assert.deepStrictEqual(run([open, ...drag([true, false]), layout(false, true)]), [
      true,
      true,
      true,
      false,
    ]);
  });

  it("elsewhere, follows the chat panel", () => {
    assert.deepStrictEqual(run([layout(false, true), layout(true, false)], false), [true, false]);
    // Its drawer state, a portable device's, is no matter.
    assert.deepStrictEqual(run([layout(false, true, true)], false), [true]);
  });
});

describe("Layout preference saver", () => {
  it("stores the swap once each time it changes, and nothing else", () => {
    const { saved, store } = recordingStore();
    const save = createPreferenceSaver(store, { swapMainPanels: false });
    const actions = createLayoutActions();
    let s = initialLayoutState;
    // The loaded state, as the provider's first effect sees it.
    save(s);
    s = actions.toggleSwapMainPanels(s);
    // An effect run twice (StrictMode's) sees the same state again.
    save(s);
    save(s);
    s = actions.toggleTheaterMode(s);
    s = actions.toggleOverlayOpen(s, true);
    save(s);
    s = actions.toggleSwapMainPanels(s, false);
    save(s);
    assert.deepStrictEqual(saved, [{ swapMainPanels: true }, { swapMainPanels: false }]);
  });

  it("stores nothing for the swap it loaded", () => {
    const { saved, store } = recordingStore();
    const save = createPreferenceSaver(store, { swapMainPanels: true });
    save({ ...initialLayoutState, swapMainPanels: true });
    assert.deepStrictEqual(saved, []);
  });
});
