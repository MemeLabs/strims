// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { ReactNode, useEffect } from "react";

import { ThreadProvider as ChatRoomProvider, useChat } from "../../contexts/Chat";

export interface RoomProviderProps {
  networkKey: Uint8Array;
  serverKey: Uint8Array;
  children: ReactNode;
}

export const RoomProvider: React.FC<RoomProviderProps> = ({ networkKey, serverKey, children }) => {
  const key = Base64.fromUint8Array(serverKey, true);

  const [{ rooms }, { openRoom }] = useChat();

  useEffect(() => openRoom(serverKey, networkKey), [Base64.fromUint8Array(networkKey, true), key]);

  if (!rooms.has(key)) {
    return null;
  }

  return (
    <ChatRoomProvider type="ROOM" topicKey={serverKey}>
      {children}
    </ChatRoomProvider>
  );
};
