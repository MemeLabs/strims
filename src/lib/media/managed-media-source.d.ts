// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// Media Source Extensions as browsers ship them beyond the DOM typings: the
// handle of a worker's MediaSource, and Safari's ManagedMediaSource, which
// plays only on a media element with remote playback disabled.

interface MediaSource {
  /** Attaches a worker's MediaSource to a window's media element as its srcObject. */
  readonly handle: MediaSourceHandle;
}

interface ManagedMediaSource extends MediaSource {
  /** The browser wants more media appended. */
  readonly streaming: boolean;
}

// A var, which globalThis and worker scopes carry as a property.
// eslint-disable-next-line no-var
declare var ManagedMediaSource:
  | {
      prototype: ManagedMediaSource;
      new (): ManagedMediaSource;
      isTypeSupported(type: string): boolean;
      readonly canConstructInDedicatedWorker: boolean;
    }
  | undefined;
