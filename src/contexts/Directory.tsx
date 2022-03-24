import { Base64 } from "js-base64";
import { omit } from "lodash/fp";
import React, { createContext, useMemo, useState } from "react";

import {
  Event,
  EventBroadcast,
  FrontendOpenResponse,
  ListingSnippet,
  Listing as directory_Listing,
} from "../apis/strims/network/v1/directory/directory";
import { useClient } from "./FrontendApi";

export interface DirectoryListing {
  id: bigint;
  listing: directory_Listing;
  snippet: ListingSnippet;
  viewerCount: number;
}

export interface DirectoryListings {
  networkKey: Uint8Array;
  listings: DirectoryListing[];
}

export type State = {
  [key: string]: DirectoryListings;
};

const initialState: State = {};

export const DirectoryContext = createContext<[State]>(null);

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [directories, setDirectories] = useState<State>(initialState);

  const setDirectoryListings = (
    key: string,
    set: (listings: DirectoryListings) => DirectoryListings
  ) =>
    setDirectories((prev) => ({
      ...prev,
      [key]: set(
        prev[key] || {
          networkKey: Base64.toUint8Array(key),
          listings: [],
        }
      ),
    }));

  const dispatchDirectoryEvent = (key: string, { events }: EventBroadcast) =>
    setDirectoryListings(key, ({ networkKey, listings }) => {
      const next = listings.slice();
      for (const { body: event } of events) {
        switch (event.case) {
          case Event.BodyCase.LISTING_CHANGE: {
            const listing: DirectoryListing = {
              id: event.listingChange.id,
              listing: event.listingChange.listing,
              snippet: event.listingChange.snippet,
              viewerCount: 0,
            };

            const i = next.findIndex((l) => l.id === event.listingChange.id);
            if (i !== -1) {
              listing.viewerCount = next[i].viewerCount;
              next[i] = listing;
            } else {
              next.push(listing);
            }
            break;
          }
          case Event.BodyCase.UNPUBLISH: {
            const i = next.findIndex((l) => l.id === event.unpublish.id);
            if (i !== -1) {
              next.splice(i, 1);
            }
            break;
          }
          case Event.BodyCase.VIEWER_COUNT_CHANGE: {
            const i = listings.findIndex((l) => l.id === event.viewerCountChange.id);
            if (i !== -1) {
              next[i] = {
                ...next[i],
                viewerCount: event.viewerCountChange.count,
              };
            }
            break;
          }
          case Event.BodyCase.VIEWER_STATE_CHANGE:
            console.log({ viewerStateChange: event.viewerStateChange });
            // TODO
            break;
          default:
            break;
        }
      }
      return { networkKey, listings: next };
    });

  const deleteDirectory = (key: string) => setDirectories((prev) => omit(key, prev));

  React.useEffect(() => {
    const events = client.directory.open();
    events.on("data", ({ networkKey, body }) => {
      switch (body.case) {
        case FrontendOpenResponse.BodyCase.BROADCAST:
          dispatchDirectoryEvent(Base64.fromUint8Array(networkKey, true), body.broadcast);
          break;
        case FrontendOpenResponse.BodyCase.CLOSE:
          deleteDirectory(Base64.fromUint8Array(networkKey, true));
          break;
      }
    });

    return () => events.destroy();
  }, []);

  const value = useMemo<[State]>(() => [directories], [directories]);

  return <DirectoryContext.Provider value={value}>{children}</DirectoryContext.Provider>;
};

Provider.displayName = "Directory.Provider";
