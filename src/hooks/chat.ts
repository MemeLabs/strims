// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useMemo, useState } from "react";

import { FrontendWatchListingUsersResponse } from "../apis/strims/network/v1/directory/directory";
import { useClient } from "../contexts/FrontendApi";

// TODO: move to directory hooks
export const useUserList = (networkKey: Uint8Array, serverKey: Uint8Array) => {
  const client = useClient();
  const [users, setUsers] = useState<Map<bigint, string>>(new Map());

  useEffect(() => {
    const events = client.directory.watchListingUsers({
      networkKey,
      query: { query: { listing: { content: { chat: { key: serverKey } } } } },
    });

    events.on("data", (e) =>
      setUsers((prev) => {
        const users = new Map(prev);
        switch (e.type) {
          case FrontendWatchListingUsersResponse.UserEventType.USER_EVENT_TYPE_JOIN:
          case FrontendWatchListingUsersResponse.UserEventType.USER_EVENT_TYPE_RENAME:
            for (const u of e.users) {
              users.set(u.id, u.alias);
            }
            break;
          case FrontendWatchListingUsersResponse.UserEventType.USER_EVENT_TYPE_PART:
            for (const u of e.users) {
              users.delete(u.id);
            }
        }
        return users;
      })
    );

    return () => events.destroy();
  }, [networkKey, serverKey]);

  return useMemo(() => Array.from(users.values()), [users]);
};
