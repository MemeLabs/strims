// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useEffect } from "react";

import RoomCarousel from "../../../components/Chat/RoomCarousel";
import { Provider as ChatProvider, useChat } from "../../../contexts/Chat";
import ChatService from "../../mocks/chat/service";
import DirectoryService from "../../mocks/directory/service";
import { MockApi, Story } from "../../storybook";

interface ContextProps {
  children: ReactNode;
}

const Context: React.FC<ContextProps> = ({ children }) => (
  <div className="chat_mockup">
    <MockApi services={() => ({ chat: new ChatService(), directory: new DirectoryService() })}>
      <ChatProvider>{children}</ChatProvider>
    </MockApi>
  </div>
);

const Carousel: React.FC = () => {
  const [, { openWhispers, setMainActiveTopic }] = useChat();

  useEffect(() => {
    const key = new Uint8Array(32);
    for (let i = 0; i < 6; i++) {
      key[0]++;
      const networkKey = key.slice();
      key[0]++;
      const serverKey = key.slice();
      openWhispers(networkKey, [serverKey]);
    }
  }, []);

  return <RoomCarousel className="chat_carousel" onChange={setMainActiveTopic} />;
};

const stories: Story[] = [
  {
    name: "Carousel",
    component: () => (
      <Context>
        <Carousel />
      </Context>
    ),
  },
];

export default stories;
