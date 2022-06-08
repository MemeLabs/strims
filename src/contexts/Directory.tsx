// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { isEqual } from "lodash";
import { omit } from "lodash/fp";
import React, { createContext, useContext, useMemo, useState } from "react";

import {
  Event,
  EventBroadcast,
  FrontendOpenResponse,
  Listing,
  ListingSnippet,
  Listing as directory_Listing,
} from "../apis/strims/network/v1/directory/directory";
import { useClient } from "./FrontendApi";

export interface DirectoryListing {
  id: bigint;
  listing: directory_Listing;
  snippet: ListingSnippet;
  viewerCount: number;
  viewers: Map<string, DirectoryUser>;
  viewersByName: Map<string, DirectoryUser>;
}

export interface DirectoryUser {
  id: bigint;
  alias: string;
  peerKey: Uint8Array;
  listingIds: bigint[];
}

export interface Directory {
  networkKey: Uint8Array;
  listings: Map<bigint, DirectoryListing>;
  users: Map<string, DirectoryUser>;
}

export const findUserMediaListing = (
  directory: Directory,
  user: DirectoryUser
): DirectoryListing => {
  for (const id of user.listingIds) {
    const listing = directory.listings.get(id);
    switch (listing.listing.content.case) {
      case Listing.ContentCase.EMBED:
      case Listing.ContentCase.MEDIA:
        return listing;
    }
  }
};

export type State = {
  [key: string]: Directory;
};

const initialState: State = {};

export interface ContextValues {
  directories: State;
}

export const DirectoryContext = createContext<ContextValues>(null);

export const useDirectory = (networkKey: Uint8Array) => {
  const { directories } = useContext(DirectoryContext);
  const key = useMemo(() => Base64.fromUint8Array(networkKey, true), [networkKey]);
  return directories[key];
};

export const useDirectoryListing = (networkKey: Uint8Array, id: bigint) => {
  return useDirectory(networkKey)?.listings.get(id);
};

type UseDirectoryUserResult = {
  user: DirectoryUser;
  networkKey: Uint8Array;
  listings: DirectoryListing[];
}[];

export const useDirectoryUser = (peerKey: Uint8Array): UseDirectoryUserResult => {
  const { directories } = useContext(DirectoryContext);
  return useMemo(() => {
    const key = Base64.fromUint8Array(peerKey, true);

    const res: UseDirectoryUserResult = [];
    for (const directory of Object.values(directories)) {
      const user = directory.users.get(key);
      if (user) {
        res.push({
          user,
          networkKey: directory.networkKey,
          listings: user.listingIds.map((id) => directory.listings.get(id)),
        });
      }
    }
    return res;
  }, [peerKey, directories]);
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
            const { id, alias, peerKey, listingIds, online } = event.viewerStateChange;
            const user: DirectoryUser = { id, alias, peerKey, listingIds };
            const key = Base64.fromUint8Array(peerKey, true);
            const prevListingIds = users.get(key)?.listingIds ?? [];

            for (const id of user.listingIds) {
              const listing = listings.get(id);
              if (listing) {
                const viewers = new Map(listing.viewers);
                const viewersByName = new Map(listing.viewersByName);
                viewers.set(key, user);
                viewersByName.set(user.alias, user);
                listings.set(id, { ...listing, viewers, viewersByName });
              }
            }

            const removedListingIds = prevListingIds.filter((id) => !user.listingIds.includes(id));
            for (const id of removedListingIds) {
              const listing = listings.get(id);
              if (listing) {
                const viewers = new Map(listing.viewers);
                const viewersByName = new Map(listing.viewersByName);
                viewers.delete(key);
                viewersByName.delete(user.alias);
                listings.set(id, { ...listing, viewers, viewersByName });
              }
            }

            if (online) {
              users.set(key, user);
            } else {
              users.delete(key);
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

  const value = useMemo<ContextValues>(() => ({ directories }), [directories]);

  return <DirectoryContext.Provider value={value}>{children}</DirectoryContext.Provider>;
};

Provider.displayName = "Directory.Provider";
