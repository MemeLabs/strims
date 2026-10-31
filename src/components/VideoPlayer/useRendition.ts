// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { MutableRefObject, useContext, useEffect, useMemo, useState } from "react";

import { Listing, ListingSnippet } from "../../apis/strims/network/v1/directory/directory";
import { PlayerContext } from "../../contexts/Player";
import { useListing } from "../../hooks/directory";
import { PlayerKind } from "../../lib/media/players";
import { RenditionTrial } from "../../lib/media/rendition-trial";
import {
  Assessment,
  Rendition,
  RenditionFailures,
  assessRenditions,
  browserProbes,
  rankRenditions,
  renditionCandidates,
  swarmId,
} from "../../lib/media/renditions";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";

/** How long the first choice waits for the stream's listing. */
export const LISTING_WAIT_MS = 1000;
/** How long the browser's answers are waited for, past which only the listing's own plays. */
export const ASSESS_WAIT_MS = 1000;
/** How long the player holds a larger size before the choice takes it. */
export const RESIZE_SETTLE_MS = 1000;

export interface RenditionProps {
  /** The listing's own stream. */
  swarmUri: string;
  mimeType: string;
  kind: PlayerKind;
  videoRef: MutableRefObject<HTMLVideoElement>;
  /** The player plays the choice on the video: its trial runs only then. */
  active: boolean;
}

export interface RenditionChoice {
  /** The stream to play, or null while the choice waits for the listing. */
  swarmUri: string | null;
  mimeType: string;
  /** The player gave up on the stream it was given. */
  unplayable: (message: string) => void;
}

/** What the viewer learned of a stream's renditions while watching it. */
interface Watched {
  /** The listing's own stream it is of. */
  main: string;
  /** The renditions that failed, and those that played, by swarm id. */
  failed: string[];
  confirmed: string[];
}

// The page's local storage, or none where the page may not use it.
const storage = (): Storage | null => {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

// Whether the device is portable: iPadOS's Safari names itself a Mac,
// which has no touch screen.
const portable = (): boolean =>
  DEVICE_TYPE === DeviceType.Portable ||
  (/Macintosh/.test(navigator.userAgent) && navigator.maxTouchPoints > 1);

// The player's picture area in device pixels: the video's box, or the
// screen's while it has none.
const measure = (video: HTMLVideoElement | undefined) => {
  const ratio = window.devicePixelRatio || 1;
  const width = video?.clientWidth || window.screen?.width || 0;
  const height = video?.clientHeight || window.screen?.height || 0;
  return { width: Math.round(width * ratio), height: Math.round(height * ratio) };
};

const label = (r: Rendition) =>
  `${Math.min(r.width, r.height)}p${Math.round(r.frameRate) || ""} ${r.video}`;

/**
 * The stream a player plays of a listing: the first of its renditions in
 * the order rankRenditions gives for this browser and the player's size,
 * leaving out those that failed. The first choice waits up to
 * LISTING_WAIT_MS for the listing's snippet, and plays the listing's own
 * without one.
 *
 * A rendition other than the listing's own is on trial until its video
 * draws a frame (RenditionTrial). One that fails its trial, or whose
 * player gives up on it (`unplayable`), is left out for the rest of the
 * stream, and a codec the browser couldn't decode is skipped on this
 * device for a while (RenditionFailures). The choice changes when one
 * fails, when the snippet's renditions change, and when the player grows:
 * the size it ranks by is the largest it held for RESIZE_SETTLE_MS, so a
 * player that shrinks keeps its rendition. Once the listing goes, the
 * renditions its last snippet named stay.
 */
export const useRendition = ({
  swarmUri,
  mimeType,
  kind,
  videoRef,
  active,
}: RenditionProps): RenditionChoice => {
  const { networkKey: listingNetwork, listingId } = useContext(PlayerContext);
  const { listing } = useListing(listingNetwork, listingId);
  const media =
    listing?.listing?.content?.case === Listing.ContentCase.MEDIA
      ? listing.listing.content.media
      : null;
  const listed =
    media && swarmId(media.swarmUri) === swarmId(swarmUri) ? listing.snippet ?? null : null;

  const [known, setKnown] = useState<{ main: string; snippet: ListingSnippet } | null>(null);
  useEffect(() => {
    if (listed) setKnown({ main: swarmUri, snippet: listed });
  }, [listed, swarmUri]);
  const snippet = listed ?? (known?.main === swarmUri ? known.snippet : null);

  const [waited, setWaited] = useState<string | null>(null);
  useEffect(() => {
    const timer = setTimeout(() => setWaited(swarmUri), LISTING_WAIT_MS);
    return () => clearTimeout(timer);
  }, [swarmUri]);

  const candidates = useMemo(
    () => renditionCandidates({ swarmUri, mimeType }, snippet),
    [swarmUri, mimeType, snippet]
  );
  const candidatesKey = useMemo(() => JSON.stringify(candidates), [candidates]);

  const probes = useMemo(() => browserProbes(kind), [kind]);
  const failures = useMemo(() => new RenditionFailures(storage(), navigator.userAgent), []);
  const [assessed, setAssessed] = useState<{ main: string; assessed: Assessment[] } | null>(null);
  useEffect(() => {
    if (candidates.length < 2) return;
    let live = true;
    let timer: ReturnType<typeof setTimeout>;
    const mainOnly = new Promise<Assessment[]>((resolve) => {
      timer = setTimeout(
        () =>
          resolve(
            candidates.map((r) => ({ rendition: r, supported: r.main, powerEfficient: false }))
          ),
        ASSESS_WAIT_MS
      );
    });
    const asked = assessRenditions(candidates, probes, (r) => failures.failed(kind, r));
    void Promise.race([asked, mainOnly]).then((a) => {
      if (live) setAssessed({ main: swarmUri, assessed: a });
    });
    return () => {
      live = false;
      clearTimeout(timer);
    };
  }, [candidatesKey, probes]);

  const [size, setSize] = useState<{ main: string; width: number; height: number } | null>(null);
  useEffect(() => {
    const video = videoRef.current;
    const grow = () =>
      setSize((s) => {
        const m = measure(video);
        const was = s?.main === swarmUri ? s : { main: swarmUri, width: 0, height: 0 };
        if (m.width <= was.width && m.height <= was.height) return was;
        return {
          main: swarmUri,
          width: Math.max(m.width, was.width),
          height: Math.max(m.height, was.height),
        };
      });
    grow();
    if (typeof ResizeObserver !== "function" || !video) return;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const observer = new ResizeObserver(() => {
      clearTimeout(timer);
      timer = setTimeout(grow, RESIZE_SETTLE_MS);
    });
    observer.observe(video);
    return () => {
      observer.disconnect();
      clearTimeout(timer);
    };
  }, [swarmUri]);

  const [watchedState, setWatched] = useState<Watched>({
    main: swarmUri,
    failed: [],
    confirmed: [],
  });
  const fresh = (s: Watched): Watched =>
    s.main === swarmUri ? s : { main: swarmUri, failed: [], confirmed: [] };
  const watched = fresh(watchedState);

  const isPortable = useMemo(portable, []);
  const choice = useMemo((): Rendition | null => {
    const main = candidates.find((r) => r.main);
    if (!snippet && waited !== swarmUri) return null;
    if (candidates.length < 2) return main;
    if (assessed?.main !== swarmUri) return null;
    const current = new Map(candidates.map((r) => [r.id, r]));
    const pool = assessed.assessed
      .filter((a) => current.has(a.rendition.id) && !watched.failed.includes(a.rendition.id))
      .map((a) => ({ ...a, rendition: current.get(a.rendition.id) }));
    const viewport = {
      width: size?.main === swarmUri ? size.width : 0,
      height: size?.main === swarmUri ? size.height : 0,
      portable: isPortable,
    };
    return rankRenditions(pool, viewport)[0] ?? main;
  }, [candidates, snippet, waited, assessed, watched, size, swarmUri, isPortable]);

  const fail = (r: Rendition) =>
    setWatched((s) => {
      const w = fresh(s);
      return w.failed.includes(r.id) ? w : { ...w, failed: [...w.failed, r.id] };
    });

  const trial = choice && !choice.main && active && !watched.confirmed.includes(choice.id);
  useEffect(() => {
    if (!trial) return;
    const r = choice;
    const t = new RenditionTrial(videoRef.current, {
      confirmed: () =>
        setWatched((s) => {
          const w = fresh(s);
          return { ...w, confirmed: [...w.confirmed, r.id] };
        }),
      failed: (reason, message) => {
        // eslint-disable-next-line no-console
        console.warn(`the rendition ${label(r)} can't play here:`, message);
        if (reason !== "media") failures.record(kind, r);
        fail(r);
      },
    });
    return () => t.destroy();
  }, [trial ? choice.id : null, trial ? choice.swarmUri : null]);

  const unplayable = useMemo(
    () => (message: string) => {
      if (!choice || choice.main) return;
      // eslint-disable-next-line no-console
      console.warn(`the rendition ${label(choice)} stopped playing:`, message);
      fail(choice);
    },
    [choice?.id, swarmUri]
  );

  return {
    swarmUri: choice?.swarmUri ?? null,
    mimeType: choice?.mimeType ?? mimeType,
    unplayable,
  };
};
