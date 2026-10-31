// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** The MIME type Safari takes an HLS source under. */
export const HLS_TYPE = "application/x-mpegURL";

/** The event a video gets when it starts or stops playing on an AirPlay receiver. */
export const WIRELESS_CHANGED = "webkitcurrentplaybacktargetiswirelesschanged";

/** The event that tells a video whether an AirPlay receiver is available. */
const AVAILABILITY_CHANGED = "webkitplaybacktargetavailabilitychanged";

/**
 * The AirPlay source of the availability watcher's video, which never
 * loads it: a video without a source it could play remotely has wireless
 * playback disabled.
 */
const WATCH_SOURCE = "airplay-availability.m3u8";

interface AirPlayVideo {
  webkitShowPlaybackTargetPicker?: () => void;
  webkitCurrentPlaybackTargetIsWireless?: boolean;
}

interface AvailabilityEvent extends Event {
  availability?: string;
}

/**
 * Whether the browser can play a ManagedMediaSource video on an AirPlay
 * receiver through an HLS alternative: Safari's ManagedMediaSource and
 * its playback target events. A ManagedMediaSource video can only do so
 * with its MediaSource in the window, attached through source children
 * (attachSources): one attached as a worker's handle doesn't open while
 * remote playback is enabled.
 */
export const airPlayCapable = (scope: object = globalThis): boolean =>
  "ManagedMediaSource" in scope && "WebKitPlaybackTargetAvailabilityEvent" in scope;

const source = (doc: Document, src: string, type: string): HTMLSourceElement => {
  const s = doc.createElement("source");
  s.type = type;
  s.src = src;
  return s;
};

/**
 * Points `video` at `src`, a window MediaSource's object URL, with
 * `alternative`, an HLS URL of the same stream, as its AirPlay source:
 * the video's two source children, in that order, with remote playback
 * enabled. Safari plays the MediaSource here, and hands a receiver the
 * HLS URL, which it doesn't fetch itself meanwhile.
 */
export const attachSources = (video: HTMLVideoElement, src: string, alternative: string): void => {
  const doc = video.ownerDocument;
  video.removeAttribute("src");
  video.replaceChildren(source(doc, src, "video/mp4"), source(doc, alternative, HLS_TYPE));
  video.disableRemotePlayback = false;
  video.load();
};

/** Removes the source children attachSources gave `video`, if it has any. */
export const detachSources = (video: HTMLVideoElement): void => {
  if (video.querySelector("source")) {
    video.replaceChildren();
    video.load();
  }
};

/**
 * Watches whether an AirPlay receiver is available, through a video of
 * its own that is never attached to the page or loaded: `changed` gets
 * each answer, the first at once. The returned function stops it.
 */
export const watchAirPlay = (
  changed: (available: boolean) => void,
  doc: Document = document
): (() => void) => {
  const video = doc.createElement("video");
  video.preload = "none";
  video.muted = true;
  video.appendChild(source(doc, WATCH_SOURCE, HLS_TYPE));
  const listener = (e: Event) => changed((e as AvailabilityEvent).availability === "available");
  video.addEventListener(AVAILABILITY_CHANGED, listener);
  return () => {
    video.removeEventListener(AVAILABILITY_CHANGED, listener);
    video.replaceChildren();
  };
};

/**
 * Whether a video on the page at `pageUrl` can name `url` as its AirPlay
 * source: Safari doesn't hand an AirPlay receiver an http alternative of
 * an https page's video.
 */
export const alternativeUsable = (url: string, pageUrl: string): boolean => {
  try {
    return !(new URL(pageUrl).protocol === "https:" && new URL(url).protocol === "http:");
  } catch {
    return false;
  }
};

/** Shows Safari's AirPlay receiver picker for `video`. */
export const showAirPlayPicker = (video: HTMLVideoElement): void =>
  (video as AirPlayVideo).webkitShowPlaybackTargetPicker?.();

/** Whether `video` plays on an AirPlay receiver. */
export const playsWireless = (video: EventTarget): boolean =>
  !!(video as AirPlayVideo).webkitCurrentPlaybackTargetIsWireless;
