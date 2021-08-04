import { Base64 } from "js-base64";
import { omit } from "lodash/fp";
import React, { createContext, useMemo, useState } from "react";

import {
  DirectoryEvent,
  DirectoryEventBroadcast,
  DirectoryFrontendOpenResponse,
  DirectoryListing,
} from "../apis/strims/network/v1/directory";
import { useClient } from "./FrontendApi";

interface Listing {
  key: string;
  listing: DirectoryListing;
}

type State = {
  [key: string]: Listing[];
};

const initialState: State = {};

export const DirectoryContext = createContext<[State]>(null);

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [directories, setDirectories] = useState<State>(initialState);

  const setDirectoryListings = (key: string, set: (listings: Listing[]) => Listing[]) =>
    setDirectories((prev) => ({
      ...prev,
      [key]: set(prev[key] || []),
    }));

  const dispatchDirectoryEvent = (key: string, { events }: DirectoryEventBroadcast) =>
    setDirectoryListings(key, (listings) =>
      events.reduce((listings, { body: event }) => {
        switch (event.case) {
          case DirectoryEvent.BodyCase.PUBLISH: {
            const listing: Listing = {
              key: Base64.fromUint8Array(event.publish.listing.key),
              listing: event.publish.listing,
            };
            return [listing, ...listings.filter((l) => l.key !== listing.key)];
          }
          case DirectoryEvent.BodyCase.UNPUBLISH: {
            const key = Base64.fromUint8Array(event.unpublish.key);
            return listings.filter((l) => l.key !== key);
          }
          default:
            return listings;
        }
      }, listings)
    );

  const deleteDirectory = (key: string) => setDirectories((prev) => omit(key, prev));

  React.useEffect(() => {
    const events = client.directory.open();
    events.on("data", ({ networkKey, body }) => {
      switch (body.case) {
        case DirectoryFrontendOpenResponse.BodyCase.BROADCAST:
          dispatchDirectoryEvent(Base64.fromUint8Array(networkKey, true), body.broadcast);
          break;
        case DirectoryFrontendOpenResponse.BodyCase.CLOSE:
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
