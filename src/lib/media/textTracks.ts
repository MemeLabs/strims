// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The cues a player shows on its video: a stream's subtitles and captions,
// in two text tracks of the video.

import type { CueEndOutput, CueOutput } from "../../../crates/strims-player/Cargo.toml";

export type { CueEndOutput, CueOutput } from "../../../crates/strims-player/Cargo.toml";

export type CueKind = CueOutput["kind"];

/** The text tracks' labels, which the CC button names. */
export const CUE_LABELS: Record<CueKind, string> = {
  subtitles: "Subtitles",
  captions: "Captions",
};

/** The event a video gets when the kinds of cues its players hold change. */
export const CUES_CHANGED = "strimscueschange";

/** Seconds of cues kept behind the playback position. */
export const CUE_BACK_BUFFER = 30;

/** The most cues a player holds; past it the oldest go. */
export const MAX_CUES = 512;

// A video's text tracks can't be removed, so each video has one track of
// each kind, made by the first player that shows a cue of it and shared by
// the players after.
const videoTracks = new WeakMap<HTMLVideoElement, Partial<Record<CueKind, TextTrack>>>();

/** The video's track of `kind`, if a player made one. */
export const cueTrack = (video: HTMLVideoElement, kind: CueKind): TextTrack | null =>
  videoTracks.get(video)?.[kind] ?? null;

/** The kinds of cues the video's tracks hold. */
export const cueKinds = (video: HTMLVideoElement): CueKind[] =>
  (["subtitles", "captions"] as const).filter((k) => !!cueTrack(video, k)?.cues?.length);

const makeTrack = (video: HTMLVideoElement, kind: CueKind): TextTrack => {
  const tracks = videoTracks.get(video) ?? {};
  let track = tracks[kind];
  if (!track) {
    track = video.addTextTrack(kind, CUE_LABELS[kind]);
    // Hidden, its cues are kept but not shown, until the viewer picks it.
    track.mode = "hidden";
    tracks[kind] = track;
    videoTracks.set(video, tracks);
  }
  return track;
};

interface Held {
  cue: VTTCue;
  track: TextTrack;
}

/**
 * A player's cues on its video. Cue times come in the player's own
 * timeline and show `shift` seconds from it on the video's (0 when the two
 * are the same). Cues may overlap: the video shows them stacked.
 */
export class CueTracks {
  private held = new Map<number, Held>();
  private shift = 0;

  constructor(private readonly video: HTMLVideoElement) {}

  /** Shows a cue. */
  add(c: CueOutput): void {
    const before = cueKinds(this.video).join();
    const track = makeTrack(this.video, c.kind);
    this.remove(c.id);
    const cue = new VTTCue(c.start + this.shift, Math.max(c.end, c.start) + this.shift, c.text);
    track.addCue(cue);
    this.held.set(c.id, { cue, track });
    while (this.held.size > MAX_CUES) {
      this.remove(this.held.keys().next().value as number);
    }
    if (cueKinds(this.video).join() !== before) this.changed();
  }

  /** Ends a cue sooner than it said. */
  end(e: CueEndOutput): void {
    const h = this.held.get(e.id);
    if (!h) return;
    h.cue.endTime = Math.max(h.cue.startTime, e.end + this.shift);
  }

  /** The video's timeline is `shift` seconds from the player's: moves the cues held. */
  setShift(shift: number): void {
    const delta = shift - this.shift;
    if (delta === 0) return;
    this.shift = shift;
    for (const { cue } of this.held.values()) {
      // Later first, so a cue's start never passes its end.
      if (delta > 0) {
        cue.endTime += delta;
        cue.startTime += delta;
      } else {
        cue.startTime += delta;
        cue.endTime += delta;
      }
    }
  }

  /** Drops the cues that ended CUE_BACK_BUFFER before the video's `currentTime`. */
  evict(currentTime: number): void {
    const before = currentTime - CUE_BACK_BUFFER;
    for (const [id, { cue }] of this.held) {
      if (cue.endTime < before) this.remove(id);
    }
  }

  /** Drops every cue held: the player's timeline starts over, or the player goes. */
  clear(): void {
    if (!this.held.size) return;
    for (const id of [...this.held.keys()]) this.remove(id);
    this.changed();
  }

  private remove(id: number): void {
    const h = this.held.get(id);
    if (!h) return;
    this.held.delete(id);
    try {
      h.track.removeCue(h.cue);
    } catch {
      // Not in the track any more.
    }
  }

  private changed(): void {
    this.video.dispatchEvent(new Event(CUES_CHANGED));
  }
}
