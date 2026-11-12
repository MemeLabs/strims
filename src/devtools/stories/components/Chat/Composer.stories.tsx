// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./chat.stories.scss";

import React, { useEffect, useRef, useState } from "react";

import { IUIConfig } from "../../../../apis/strims/chat/v1/chat";
import Composer from "../../../../components/Chat/Composer";
import { useChat, useRoom } from "../../../../contexts/Chat";
import { MockChat, RoomProvider, RoomStyles } from "../../../contexts/Chat";
import { replay } from "../../../mocks/chat/MessageEmitter";
import ChatService from "../../../mocks/chat/service";
import { Story } from "../../../storybook";

// Types `text` into the composer's editor as the keyboard would: the caret
// placed in it (the selectionchange Slate reads it on, which its beforeinput
// handler flushes), the beforeinput Slate inserts from, then a key press,
// which restarts the composer's slow typing timer: the autocomplete shows
// once it fires. (execCommand fires no beforeinput, so Slate never sees it.)
const typeInComposer = (root: HTMLElement, text: string): void => {
  const editor = root.querySelector<HTMLElement>("[data-slate-editor]");
  editor.focus();
  // A page without the window's focus (headless, a background tab) gets no
  // focus events, and the autocomplete shows only while focused.
  editor.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  const leaf = editor.querySelector("[data-slate-zero-width], [data-slate-string]");
  window.getSelection().collapse(leaf.firstChild, 0);
  document.dispatchEvent(new Event("selectionchange"));
  editor.dispatchEvent(
    new InputEvent("beforeinput", {
      inputType: "insertText",
      data: text,
      bubbles: true,
      cancelable: true,
    })
  );
  editor.dispatchEvent(new KeyboardEvent("keydown", { key: "Shift", bubbles: true }));
};

interface ComposerViewProps {
  /** Runs once the room's emotes and the emoji have arrived. */
  onReady?: (el: HTMLElement) => void;
}

const ComposerView: React.FC<ComposerViewProps> = ({ onReady }) => {
  const [{ emoji }] = useChat();
  const [room] = useRoom();
  const [sent, setSent] = useState<string[]>([]);
  const ref = useRef<HTMLDivElement>(null);
  const ready = !!emoji && room.nicks.length > 0;

  useEffect(() => {
    if (ready) {
      onReady?.(ref.current);
    }
  }, [ready]);

  return (
    <div className="chat_story__panel" ref={ref}>
      <div className="chat_story__filler">
        {sent.map((body, i) => (
          <p key={i} className="chat_story__caption">
            sent: {body}
          </p>
        ))}
      </div>
      <div className="chat__footer">
        <Composer
          emotes={room.emotes}
          modifiers={room.modifiers}
          tags={room.tags}
          nicks={room.nicks}
          commands={room.commands}
          onMessage={(body) => setSent((prev) => [...prev, body])}
          // Shown soon after the story types, so it's up when it's captured.
          slowTypingDelay={50}
        />
      </div>
    </div>
  );
};

interface ComposerStoryProps extends ComposerViewProps {
  uiConfig?: Partial<IUIConfig>;
}

// The composer at the foot of the stories' room, its nicks, emotes,
// modifiers and tags the room's; what it sends is listed above it.
const ComposerStory: React.FC<ComposerStoryProps> = ({ uiConfig, ...props }) => (
  <MockChat chat={() => new ChatService(replay([]), { uiConfig })}>
    <RoomProvider>
      <RoomStyles className="chat_story chat_story--composer" waitForEmotes>
        <ComposerView {...props} />
      </RoomStyles>
    </RoomProvider>
  </MockChat>
);

const type = (text: string) => (el: HTMLElement) => typeInComposer(el, text);

const stories: Story[] = [
  { name: "Empty", component: () => <ComposerStory /> },
  {
    name: "With text",
    component: () => (
      <ComposerStory
        onReady={type(
          "PeepoRun hey soadsoap check strims.gg `code here` ||a spoiler|| nsfw FeelsGoodMan:wide 🍕 "
        )}
      />
    ),
  },
  {
    name: "Greentext",
    component: () => <ComposerStory onReady={type(">implying it works PEPE ")} />,
  },
  { name: "Self", component: () => <ComposerStory onReady={type("/me waves NODDERS ")} /> },
  { name: "Emote autocomplete", component: () => <ComposerStory onReady={type("look Fee")} /> },
  { name: "Modifier autocomplete", component: () => <ComposerStory onReady={type("PEPE:w")} /> },
  { name: "Nick autocomplete", component: () => <ComposerStory onReady={type("hey @ka")} /> },
  { name: "Command autocomplete", component: () => <ComposerStory onReady={type("/h")} /> },
  { name: "Emoji autocomplete", component: () => <ComposerStory onReady={type(":pizz")} /> },
  {
    name: "Autocomplete off",
    component: () => (
      <ComposerStory uiConfig={{ autocompleteHelper: false }} onReady={type("look Fee")} />
    ),
  },
  {
    name: "No emote previews",
    component: () => (
      <ComposerStory uiConfig={{ autocompleteEmotePreview: false }} onReady={type("look Fee")} />
    ),
  },
  {
    name: "Emote menu",
    component: () => (
      <ComposerStory
        onReady={(el) => el.querySelector<HTMLElement>(".chat_composer__button").click()}
      />
    ),
  },
  {
    name: "Long text",
    component: () => (
      <ComposerStory
        onReady={type(
          `${"a".repeat(
            120
          )} and then a long sentence that wraps over several lines of the composer, PeepoRun included, before it ends`
        )}
      />
    ),
  },
];

export default stories;
