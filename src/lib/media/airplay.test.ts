// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import {
  HLS_TYPE,
  airPlayCapable,
  attachSources,
  detachSources,
  playsWireless,
  watchAirPlay,
} from "./airplay";

// The parts of an element the AirPlay helpers touch.
class FakeElement extends EventTarget {
  children: FakeElement[] = [];
  attributes = new Map<string, string>();
  loads = 0;
  disableRemotePlayback = true;
  preload = "auto";
  muted = false;
  type = "";
  src = "";
  webkitCurrentPlaybackTargetIsWireless = false;

  constructor(readonly tag: string, readonly ownerDocument: FakeDocument) {
    super();
  }

  appendChild(c: FakeElement) {
    this.children.push(c);
    return c;
  }

  replaceChildren(...c: FakeElement[]) {
    this.children = c;
  }

  querySelector(tag: string) {
    return this.children.find((c) => c.tag === tag) ?? null;
  }

  removeAttribute(name: string) {
    this.attributes.delete(name);
  }

  load() {
    this.loads++;
  }
}

class FakeDocument {
  made: FakeElement[] = [];

  createElement(tag: string) {
    const e = new FakeElement(tag, this);
    this.made.push(e);
    return e;
  }
}

const video = () => {
  const doc = new FakeDocument();
  return { doc, v: doc.createElement("video") };
};

describe("airplay", () => {
  it("is possible where Safari's ManagedMediaSource and playback target events are", () => {
    assert.ok(
      airPlayCapable({ ManagedMediaSource: {}, WebKitPlaybackTargetAvailabilityEvent: {} })
    );
    assert.ok(!airPlayCapable({ ManagedMediaSource: {} }));
    assert.ok(!airPlayCapable({ MediaSource: {}, WebKitPlaybackTargetAvailabilityEvent: {} }));
  });

  it("attaches a MediaSource with the HLS URL as its alternative, remote playback on", () => {
    const { v } = video();
    v.attributes.set("src", "blob:old");
    attachSources(v as unknown as HTMLVideoElement, "blob:1", "https://node.example/x.m3u8");
    assert.ok(!v.attributes.has("src"));
    assert.deepStrictEqual(
      v.children.map((c) => [c.tag, c.type, c.src]),
      [
        ["source", "video/mp4", "blob:1"],
        ["source", HLS_TYPE, "https://node.example/x.m3u8"],
      ]
    );
    assert.strictEqual(v.disableRemotePlayback, false);
    assert.strictEqual(v.loads, 1);
    // A new MediaSource takes the old one's place.
    attachSources(v as unknown as HTMLVideoElement, "blob:2", "https://node.example/x.m3u8");
    assert.deepStrictEqual(
      v.children.map((c) => c.src),
      ["blob:2", "https://node.example/x.m3u8"]
    );
    detachSources(v as unknown as HTMLVideoElement);
    assert.deepStrictEqual(v.children, []);
    assert.strictEqual(v.loads, 3);
    detachSources(v as unknown as HTMLVideoElement);
    assert.strictEqual(v.loads, 3, "a video without sources is left alone");
  });

  it("watches availability through a video of its own that names an AirPlay source but never loads", () => {
    const doc = new FakeDocument();
    const answers: boolean[] = [];
    const stop = watchAirPlay((a) => answers.push(a), doc as unknown as Document);
    const [watcher] = doc.made;
    assert.strictEqual(watcher.preload, "none");
    assert.deepStrictEqual(
      watcher.children.map((c) => c.type),
      [HLS_TYPE]
    );
    const changed = (availability: string) =>
      watcher.dispatchEvent(
        Object.assign(new Event("webkitplaybacktargetavailabilitychanged"), { availability })
      );
    changed("not-available");
    changed("available");
    stop();
    changed("not-available");
    assert.deepStrictEqual(answers, [false, true]);
    assert.strictEqual(watcher.loads, 0);
  });

  it("reads whether a video plays on a receiver", () => {
    const { v } = video();
    assert.ok(!playsWireless(v));
    v.webkitCurrentPlaybackTargetIsWireless = true;
    assert.ok(playsWireless(v));
  });
});
