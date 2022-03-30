import { Base64 } from "js-base64";
import { omit } from "lodash/fp";
import React, { createContext, useContext, useMemo, useState } from "react";

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
  viewers: Map<bigint, DirectoryUser>;
  viewersByName: Map<string, DirectoryUser>;
}

export interface DirectoryUser {
  id: bigint;
  alias: string;
  viewingIds: bigint[];
}

export interface Directory {
  networkKey: Uint8Array;
  listings: Map<bigint, DirectoryListing>;
  users: Map<bigint, DirectoryUser>;
}

export type State = {
  [key: string]: Directory;
};

const initialState: State = {};

export const DirectoryContext = createContext<[State]>(null);

export const useDirectory = (networkKey: Uint8Array) => {
  const [state] = useContext(DirectoryContext);
  const key = useMemo(() => Base64.fromUint8Array(networkKey, true), [networkKey]);
  return state[key];
};

export const useDirectoryListing = (networkKey: Uint8Array, id: bigint) => {
  return useDirectory(networkKey)?.listings.get(id);
};

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [directories, setDirectories] = useState<State>(initialState);

  const setDirectoryListings = (key: string, set: (listings: Directory) => Directory) =>
    setDirectories((prev) => ({
      ...prev,
      [key]: set(
        prev[key] || {
          networkKey: Base64.toUint8Array(key),
          listings: new Map(),
          users: new Map(),
        }
      ),
    }));

  const dispatchDirectoryEvent = (key: string, { events }: EventBroadcast) =>
    setDirectoryListings(key, ({ networkKey, listings: prevListings, users: prevUsers }) => {
      const listings = new Map(prevListings);
      const users = new Map(prevUsers);
      for (const { body: event } of events) {
        switch (event.case) {
          case Event.BodyCase.LISTING_CHANGE: {
            const { id, listing, snippet } = event.listingChange;
            listings.set(id, {
              id,
              viewerCount: 0,
              viewers: new Map(),
              viewersByName: new Map(),
              ...listings.get(id),
              listing,
              snippet,
            });
            break;
          }
          case Event.BodyCase.UNPUBLISH: {
            listings.delete(event.unpublish.id);
            break;
          }
          case Event.BodyCase.VIEWER_COUNT_CHANGE: {
            const { id, count } = event.viewerCountChange;
            const prevListing = listings.get(id);
            if (prevListing) {
              listings.set(id, {
                ...prevListing,
                viewerCount: count,
              });
            }
            break;
          }
          case Event.BodyCase.VIEWER_STATE_CHANGE: {
            const { id, alias, viewingIds, online } = event.viewerStateChange;
            const user: DirectoryUser = { id, alias, viewingIds };
            const prevViewingIds = users.get(user.id)?.viewingIds ?? [];

            for (const id of user.viewingIds) {
              const listing = listings.get(id);
              if (listing) {
                const viewers = new Map(listing.viewers);
                const viewersByName = new Map(listing.viewersByName);
                viewers.set(user.id, user);
                viewersByName.set(user.alias, user);
                listings.set(id, { ...listing, viewers, viewersByName });
              }
            }

            const removedViewingIds = prevViewingIds.filter((id) => !user.viewingIds.includes(id));
            for (const id of removedViewingIds) {
              const listing = listings.get(id);
              if (listing) {
                const viewers = new Map(listing.viewers);
                const viewersByName = new Map(listing.viewersByName);
                viewers.delete(user.id);
                viewersByName.delete(user.alias);
                listings.set(id, { ...listing, viewers, viewersByName });
              }
            }

            if (online) {
              users.set(user.id, user);
            } else {
              users.delete(user.id);
            }
            break;
          }
        }
      }
      return { networkKey, listings, users };
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
