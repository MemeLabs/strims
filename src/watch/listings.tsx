// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, createContext, useContext, useEffect, useState } from "react";

import {
  FrontendWatchListingsResponse,
  ListingSnippetImage,
} from "../apis/strims/network/v1/directory/directory";
import { readDelimited } from "../lib/delimited";
import { NetworkListingsMap, foldListingEvents } from "../lib/listings";
import { apiUrl, eventsUrl } from "./api";

/** The stream pings every 15 s; one silent for this long has gone. */
const IDLE_TIMEOUT_MS = 45 * 1000;
const MIN_BACKOFF_MS = 1000;
const MAX_BACKOFF_MS = 30 * 1000;
/** A stream that lasted this long resets the backoff. */
const HEALTHY_MS = 30 * 1000;

export interface BootstrapListings {
  networkListings: NetworkListingsMap;
  /** Whether a stream is open. */
  connected: boolean;
  error: Error | null;
}

const initialListings: BootstrapListings = {
  networkListings: new Map(),
  connected: false,
  error: null,
};

// Points an image the API names by path at the API.
const resolveImage = (image: ListingSnippetImage | undefined) => {
  if (image?.sourceOneof.case === ListingSnippetImage.SourceOneofCase.URL) {
    image.sourceOneof = new ListingSnippetImage.SourceOneof({
      url: apiUrl(image.sourceOneof.url),
    });
  }
};

const resolveImages = (events: FrontendWatchListingsResponse.Event[]) => {
  for (const { event } of events) {
    if (event.case === FrontendWatchListingsResponse.Event.EventCase.CHANGE) {
      for (const { snippet } of event.change.listings.listings) {
        resolveImage(snippet?.thumbnail);
        resolveImage(snippet?.channelLogo);
      }
    }
  }
  return events;
};

const sleep = (ms: number, signal: AbortSignal) =>
  new Promise<void>((resolve) => {
    const onAbort = () => {
      clearTimeout(timer);
      resolve();
    };
    const timer = setTimeout(() => {
      signal.removeEventListener("abort", onAbort);
      resolve();
    }, ms);
    signal.addEventListener("abort", onAbort, { once: true });
  });

/**
 * Follows the native node's directory projection while mounted, reading
 * it again with backoff whenever it ends. Each stream starts the listings
 * over: its first message replaces what the last one left, and a stream
 * that fails before its first clears them.
 */
const useBootstrapListingsStream = (): BootstrapListings => {
  const [listings, setListings] = useState(initialListings);

  useEffect(() => {
    const aborter = new AbortController();
    const { signal } = aborter;
    void (async () => {
      let backoff = MIN_BACKOFF_MS;
      while (!signal.aborted) {
        const started = Date.now();
        let first = true;
        let error: Error | null = null;
        try {
          await readDelimited(eventsUrl(), {
            signal,
            idleTimeoutMs: IDLE_TIMEOUT_MS,
            onMessage: (message) => {
              const { events } = FrontendWatchListingsResponse.decode(message);
              const fresh = first;
              first = false;
              setListings((prev) => ({
                connected: true,
                error: null,
                networkListings: foldListingEvents(
                  fresh ? new Map() : prev.networkListings,
                  resolveImages(events)
                ),
              }));
            },
          });
        } catch (e) {
          error = e instanceof Error ? e : new Error(String(e));
        }
        if (signal.aborted) break;
        const cleared = first;
        setListings((prev) => ({
          connected: false,
          error,
          networkListings: cleared ? new Map() : prev.networkListings,
        }));

        if (Date.now() - started >= HEALTHY_MS) backoff = MIN_BACKOFF_MS;
        await sleep(backoff * (0.5 + Math.random() / 2), signal);
        backoff = Math.min(backoff * 2, MAX_BACKOFF_MS);
      }
    })();
    return () => aborter.abort();
  }, []);

  return listings;
};

const BootstrapListingsContext = createContext<BootstrapListings>(initialListings);

/** Holds one stream of the native node's listings for every page under it. */
export const BootstrapListingsProvider: React.FC<{ children: ReactNode }> = ({ children }) => (
  <BootstrapListingsContext.Provider value={useBootstrapListingsStream()}>
    {children}
  </BootstrapListingsContext.Provider>
);

export const useBootstrapListings = (): BootstrapListings => useContext(BootstrapListingsContext);
