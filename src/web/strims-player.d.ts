// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The wasm-bindgen glue for crates/strims-player, which
// vendor_modules/strims-wasm-loader builds when the player worker imports the
// crate's Cargo.toml. crates/strims-player/tests/wasm.rs checks the objects'
// shapes.
declare module "*/strims-player/Cargo.toml" {
  /** Fetches and instantiates the module, once per worker, before any Remuxer is made. */
  export default function init(): Promise<unknown>;

  /**
   * Remuxes one `video/x.strims-units` stream, as the Egress RPC's `Data`
   * messages carry it, into the fMP4 one SourceBuffer appends: an init
   * segment per set of decoder configs, then a fragment per unit.
   *
   * Outputs come in append order. Nothing is written out at a loss: each
   * track's unit before it lasts until its next one, so the picture holds
   * until the next keyframe while audio plays on, and the buffered timeline
   * has no hole. A writer session that starts its timestamps over is moved
   * to follow the output (a `restart` output), so decode times never go back
   * and no `timestampOffset` is needed.
   *
   * Recovery: after a fatal media error, rebuild the MediaSource and make a
   * new Remuxer, which starts with an init at the next GOP's configs. To go
   * on across a reopened Egress stream, push the new stream's first message
   * with `discontinuity` set.
   */
  export class Remuxer {
    constructor();
    /**
     * Takes one `Data` message: its bytes, `segmentEnd` and `discontinuity`.
     * `data` is copied during the call; the caller keeps it and may transfer
     * it afterwards. Answers the outputs the message completed, in order.
     * No input throws (what it can't use is counted in `metrics()`); a throw
     * is the module trapping (its memory can't grow), after which the worker
     * should be discarded.
     */
    push(data: Uint8Array, segmentEnd: boolean, discontinuity: boolean): RemuxerOutput[];
    /** The stream ended: the outputs of the units it holds. */
    finish(): RemuxerOutput[];
    metrics(): RemuxerMetrics;
    /** Frees its memory now (garbage collection also does); later calls throw. */
    free(): void;
  }

  export type RemuxerOutput =
    | InitOutput
    | MediaOutput
    | GapOutput
    | DiscontinuityOutput
    | RestartOutput;

  /**
   * An init segment (ftyp, moov), for the media after it. It comes at a
   * keyframe of the cut track (video, or audio without video) once the
   * stream's configs change; one no different from the last isn't output.
   */
  export interface InitOutput {
    type: "init";
    /** A new ArrayBuffer of exactly these bytes: the caller's, transferable. */
    data: Uint8Array<ArrayBuffer>;
    /**
     * The SourceBuffer's type, e.g. `video/mp4;codecs="avc1.64001f,mp4a.40.2"`;
     * `audio/mp4` without video; an Opus track's codec is `Opus`. Call
     * `changeType(mime)` before appending an init after the first (Firefox
     * needs it when the codecs change).
     */
    mime: string;
    /** The tracks it declares, in its order: video first, then audio, each by id. */
    tracks: TrackInfo[];
    /**
     * The previous init's SourceBuffer takes it: as many tracks of each kind,
     * with the same ids for a kind with more than one. False for the first
     * init; otherwise false means a new MediaSource.
     */
    compatible: boolean;
  }

  export type TrackInfo = VideoTrackInfo | AudioTrackInfo;

  export interface VideoTrackInfo {
    id: number;
    kind: "video";
    /** The RFC 6381 codec string, e.g. `avc1.64001f`. */
    codec: string;
    /** Units per second of the track's timestamps. */
    timescale: number;
    /** The picture size after cropping, in pixels. */
    width: number;
    height: number;
  }

  export interface AudioTrackInfo {
    id: number;
    kind: "audio";
    /** The RFC 6381 codec string, e.g. `mp4a.40.2`, or `Opus`. */
    codec: string;
    timescale: number;
    /** The output sample rate, and the channel count (0: a program config element sets it). */
    sampleRate: number;
    channels: number;
  }

  /**
   * One fragment (prft, moof, mdat). A unit is written once its track's next
   * unit gives its duration. Consecutive media outputs may be appended as one
   * buffer.
   */
  export interface MediaOutput {
    type: "media";
    /** A new ArrayBuffer of exactly these bytes: the caller's, transferable. */
    data: Uint8Array<ArrayBuffer>;
    /** The units it holds, per track in the init's order, each track's in decode order. */
    units: UnitInfo[];
  }

  export interface UnitInfo {
    track: number;
    kind: "video" | "audio";
    keyframe: boolean;
    /**
     * When it presents on the SourceBuffer's timeline, in seconds: its PTS
     * with the init's edit list applied (as browsers apply it), moved past
     * any restart, before any `timestampOffset`. A stream's first B-frames
     * can be a little below 0.
     */
    pts: number;
    /** Its publish time on the writer's Unix clock, in milliseconds. */
    published: number;
  }

  /**
   * `missing` units of `track` never arrived, by their sequence numbers. The
   * track's unit before them lasts until its unit after them; video resumes
   * at its next keyframe.
   */
  export interface GapOutput {
    type: "gap";
    track: number;
    kind: "video" | "audio";
    missing: number;
  }

  /** The input skipped (the message had `discontinuity`): the record in progress was dropped. */
  export interface DiscontinuityOutput {
    type: "discontinuity";
  }

  /**
   * The writer's session started over at earlier timestamps. Units from here
   * on are moved by `offset` seconds (the whole offset, not its change), so
   * they follow the output so far.
   */
  export interface RestartOutput {
    type: "restart";
    offset: number;
  }

  /** Counts since the Remuxer was made, and what it holds now. */
  export interface RemuxerMetrics {
    /** Messages pushed, and their bytes. */
    messages: number;
    bytes: number;
    /** Records read whole, dropped for passing 8 MiB, and dropped unfinished at a discontinuity. */
    records: number;
    oversizeRecords: number;
    partialRecords: number;
    discontinuities: number;
    /** Units given to the fragment writer. */
    units: number;
    /** Units dropped before the first init, while their track's new config waits for its init, and for going back. */
    droppedBeforeInit: number;
    droppedPendingConfig: number;
    droppedBackwards: number;
    /** Configs no init segment could declare. */
    refusedInits: number;
    inits: number;
    fragments: number;
    outputBytes: number;
    restarts: number;
    /** Times the bounds on held units (8 MiB, 4096 units) wrote them all out. */
    heldFlushes: number;
    /** Payload bytes held unwritten now. */
    heldBytes: number;
    /** The module's linear memory, in bytes; it only grows. */
    memoryBytes: number;
    /** The unit reader's counts. */
    reader: {
      records: number;
      units: number;
      configs: number;
      gaps: number;
      missing: number;
      awaitingKeyframe: number;
      configMismatches: number;
      unknownTracks: number;
      stale: number;
      outOfOrder: number;
      malformed: number;
      unknownKinds: number;
      unknownEntries: number;
      lost: number;
    };
  }
}
