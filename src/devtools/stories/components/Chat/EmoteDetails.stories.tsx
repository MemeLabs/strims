// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React, { useState } from "react";

import EmoteDetails from "../../../../components/Chat/EmoteDetails";
import { MockChat, RoomProvider, RoomStyles } from "../../../contexts/Chat";
import { replay } from "../../../mocks/chat/MessageEmitter";
import ChatService from "../../../mocks/chat/service";
import { Story } from "../../../storybook";

interface DetailsProps {
  name: string;
  modifiers?: string[];
}

// The details of `name` as clicking it in a message opens them, once the
// room's emotes have arrived (EmoteDetails needs the emote among them). Close
// shows how to open them again.
const Details: React.FC<DetailsProps> = ({ name, modifiers = [] }) => {
  const [open, setOpen] = useState(true);
  return (
    <MockChat chat={() => new ChatService(replay([]))}>
      <RoomProvider>
        <RoomStyles className="chat_story" waitForEmotes>
          <p className="chat_story__caption">
            {open ? `details of ${name}` : "closed; "}
            {!open && (
              <button className="input input_button" onClick={() => setOpen(true)}>
                open again
              </button>
            )}
          </p>
          {open && (
            <EmoteDetails
              name={name}
              modifiers={modifiers}
              anchor={[160, 110]}
              onClose={() => setOpen(false)}
            />
          )}
        </RoomStyles>
      </RoomProvider>
    </MockChat>
  );
};

const stories: Story[] = [
  { name: "Contributor", component: () => <Details name="4Head" /> },
  { name: "No contributor", component: () => <Details name="PEPE" /> },
  {
    name: "Modifiers",
    component: () => <Details name="FeelsGoodMan" modifiers={["wide", "flip"]} />,
  },
  { name: "Animated", component: () => <Details name="PeepoRun" modifiers={["jam"]} /> },
  { name: "Wide emote", component: () => <Details name="Abathur" /> },
  {
    // Details of an emote the room's bundle no longer has close themselves.
    name: "Removed",
    component: () => <Details name="NoLongerAnEmote" />,
  },
];

export default stories;
