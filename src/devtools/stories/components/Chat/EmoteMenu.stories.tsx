// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React, { useEffect, useRef, useState } from "react";

import { IUIConfig } from "../../../../apis/strims/chat/v1/chat";
import EmoteMenu from "../../../../components/Chat/EmoteMenu";
import { useChat } from "../../../../contexts/Chat";
import { MockChat, RoomProvider, RoomStyles, typeInto } from "../../../contexts/Chat";
import { replay } from "../../../mocks/chat/MessageEmitter";
import ChatService from "../../../mocks/chat/service";
import { Story } from "../../../storybook";

interface MenuProps {
  /** Runs once the emotes and emoji have arrived, with the menu's element. */
  onReady?: (el: HTMLElement) => void;
}

const Menu: React.FC<MenuProps> = ({ onReady }) => {
  const [{ emoji }] = useChat();
  const [selected, setSelected] = useState("");
  const ref = useRef<HTMLDivElement>(null);
  const ready = !!emoji;

  useEffect(() => {
    if (ready) {
      onReady?.(ref.current);
    }
  }, [ready]);

  return (
    <div className="chat_story chat_story--emote_menu" ref={ref}>
      <EmoteMenu
        onSelect={(v, send) => setSelected(`${send ? "sent" : "inserted"} ${v}`)}
        onClose={() => setSelected("closed")}
      />
      {selected && <p className="chat_story__caption">{selected}</p>}
    </div>
  );
};

interface MenuStoryProps extends MenuProps {
  uiConfig?: Partial<IUIConfig>;
  /** Whether the room has emotes (else emoji alone). */
  emotes?: boolean;
}

const MenuStory: React.FC<MenuStoryProps> = ({ uiConfig, emotes = true, ...props }) => (
  <MockChat
    chat={() => new ChatService(replay([]), { uiConfig, assetBundle: emotes ? undefined : null })}
  >
    <RoomProvider>
      <RoomStyles waitForEmotes={emotes}>
        <Menu {...props} />
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

const search = (query: string) => (el: HTMLElement) =>
  typeInto(el.querySelector<HTMLInputElement>(".emote_menu_search__input"), query);

// Moves the pointer onto the first `selector` (with `text`, if given) once
// it's there: the scroller lays its categories out after measuring itself,
// and a search after its debounce. Gives up after 3 s.
const hover =
  (selector: string, text?: string) =>
  (el: HTMLElement, tries = 60): void => {
    const target = Array.from(el.querySelectorAll(selector)).find(
      (e) => text === undefined || e.textContent === text
    );
    if (target) {
      target.dispatchEvent(
        new MouseEvent("mouseover", { bubbles: true, relatedTarget: document.body })
      );
    } else if (tries > 0) {
      window.setTimeout(() => hover(selector, text)(el, tries - 1), 50);
    }
  };

const stories: Story[] = [
  { name: "Default", component: () => <MenuStory /> },
  { name: "Search", component: () => <MenuStory onReady={search("pepe")} /> },
  { name: "No results", component: () => <MenuStory onReady={search("zzzzzz")} /> },
  {
    name: "Preview",
    component: () => (
      <MenuStory onReady={(el) => hover(".emote_menu__category__list_item--emote")(el)} />
    ),
  },
  {
    name: "Emoji preview",
    component: () => (
      <MenuStory
        onReady={(el) => {
          search("pizza")(el);
          hover(".emote_menu__category__list_item--emoji", "🍕")(el);
        }}
      />
    ),
  },
  {
    name: "Skin tone",
    component: () => (
      <MenuStory uiConfig={{ emojiSkinTone: "\u{1F3FD}" }} onReady={search("hand")} />
    ),
  },
  {
    name: "Skin tones open",
    component: () => (
      <MenuStory
        onReady={(el) =>
          el.querySelector<HTMLElement>(".emote_menu_skin_tones__dropdown__anchor")?.click()
        }
      />
    ),
  },
  { name: "No emotes", component: () => <MenuStory emotes={false} /> },
];

export default stories;
