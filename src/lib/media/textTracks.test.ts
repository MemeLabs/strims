// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import assert from "assert";

import { CUES_CHANGED, CUE_BACK_BUFFER, CueTracks, cueKinds, cueTrack } from "./textTracks";

export class FakeCue {
  constructor(public startTime: number, public endTime: number, public text: string) {}
}

export class FakeTextTrack {
  mode = "disabled";
  list: FakeCue[] = [];
  constructor(public kind: string, public label: string) {}
  get cues() {
    return this.mode === "disabled" ? null : this.list;
  }
  addCue(c: FakeCue) {
    this.list.push(c);
  }
  removeCue(c: FakeCue) {
    const i = this.list.indexOf(c);
    if (i < 0) throw new Error("NotFoundError");
    this.list.splice(i, 1);
  }
}

export class FakeTextVideo extends EventTarget {
  tracks: FakeTextTrack[] = [];
  addTextTrack(kind: string, label: string) {
    const t = new FakeTextTrack(kind, label);
    t.mode = "hidden";
    this.tracks.push(t);
    return t;
  }
}

const globals = globalThis as unknown as Record<string, unknown>;

const cue = (id: number, kind: "subtitles" | "captions", start: number, end: number, text = "x") =>
  ({ type: "cue", id, kind, start, end, text } as const);

const times = (t: TextTrack | null) =>
  Array.from((t as unknown as FakeTextTrack).list, (c) => [c.startTime, c.endTime, c.text]);

describe("CueTracks", () => {
  before(() => {
    globals.VTTCue = FakeCue;
  });
  after(() => {
    delete globals.VTTCue;
  });

  it("shows cues of each kind in a track of its own, made once a video and shared", () => {
    const video = new FakeTextVideo();
    const v = video as unknown as HTMLVideoElement;
    let changes = 0;
    video.addEventListener(CUES_CHANGED, () => changes++);
    const a = new CueTracks(v);
    a.add(cue(1, "subtitles", 1, 3, "one"));
    a.add(cue(2, "subtitles", 2, 4, "two"));
    assert.deepStrictEqual(cueKinds(v), ["subtitles"]);
    assert.strictEqual(changes, 1);
    a.add(cue(3, "captions", 2, 18, "CC"));
    assert.deepStrictEqual(cueKinds(v), ["subtitles", "captions"]);
    assert.strictEqual(changes, 2);
    assert.deepStrictEqual(
      video.tracks.map((t) => [t.kind, t.label, t.mode]),
      [
        ["subtitles", "Subtitles", "hidden"],
        ["captions", "Captions", "hidden"],
      ]
    );
    // Overlapping cues stay as they are; an end shortens one.
    a.end({ type: "cueEnd", id: 3, end: 5 });
    assert.deepStrictEqual(times(cueTrack(v, "subtitles")), [
      [1, 3, "one"],
      [2, 4, "two"],
    ]);
    assert.deepStrictEqual(times(cueTrack(v, "captions")), [[2, 5, "CC"]]);
    // A later player of the video takes its tracks.
    a.clear();
    assert.deepStrictEqual(cueKinds(v), []);
    const b = new CueTracks(v);
    b.add(cue(1, "subtitles", 9, 10));
    assert.strictEqual(video.tracks.length, 2);
    assert.deepStrictEqual(times(cueTrack(v, "subtitles")), [[9, 10, "x"]]);
  });

  it("moves its cues by the shift between timelines", () => {
    const video = new FakeTextVideo();
    const v = video as unknown as HTMLVideoElement;
    const c = new CueTracks(v);
    c.setShift(10);
    c.add(cue(1, "subtitles", 1, 2));
    c.setShift(7.5);
    c.add(cue(2, "subtitles", 3, 4));
    c.end({ type: "cueEnd", id: 1, end: 1.5 });
    assert.deepStrictEqual(times(cueTrack(v, "subtitles")), [
      [8.5, 9, "x"],
      [10.5, 11.5, "x"],
    ]);
  });

  it("drops cues that ended past the back buffer", () => {
    const video = new FakeTextVideo();
    const v = video as unknown as HTMLVideoElement;
    const c = new CueTracks(v);
    c.add(cue(1, "subtitles", 0, 1));
    c.add(cue(2, "subtitles", 10, 11));
    c.evict(CUE_BACK_BUFFER + 5);
    assert.deepStrictEqual(times(cueTrack(v, "subtitles")), [[10, 11, "x"]]);
  });
});
