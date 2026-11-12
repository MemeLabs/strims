// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../components/Layout/Layout.scss";
import "./Layout.stories.scss";

import React, { ReactNode, useEffect, useRef } from "react";
import { Route } from "react-router-dom";

import Header from "../../../../components/Layout/Header";
import DirectoryService from "../../../mocks/directory/service";
import { MockApi, StaticPlayer, Story, StoryRouter } from "../../../storybook";
import { DebugService } from "../Debugger/metrics";
import { usePortletPlacement } from "../Debugger/portlet";
import { LayoutState } from "./shell";

// Focuses the header's search and types `text`, as a viewer would (a page
// without focus moves it without a focus event: React hears one anyway).
const useSearch = (root: React.RefObject<HTMLElement>, text?: string) =>
  useEffect(() => {
    if (text === undefined) {
      return;
    }
    const input = root.current.querySelector<HTMLInputElement>(".search__input");
    input.focus();
    if (!document.hasFocus()) {
      input.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
    }
    Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, text);
    input.dispatchEvent(new Event("input", { bubbles: true }));
  }, []);

// Opens the activity monitor (the debugger) as its button would, and moves
// its window below the header.
const OpenDebugger: React.FC<{ root: React.RefObject<HTMLElement> }> = ({ root }) => {
  useEffect(() => {
    root.current.querySelectorAll<HTMLElement>(".layout_header__user_nav__link")[1].click();
  }, []);
  usePortletPlacement(() => document.querySelector(".debugger__tabs"), { y: 66 });
  return null;
};

interface HeaderStoryProps {
  search?: ReactNode;
  showChat?: boolean;
  query?: string;
  debug?: boolean;
}

// The header in the layout's header row. Its links, the search's results
// and the settings modal (which navigates to /settings) stay in the story.
const HeaderStory: React.FC<HeaderStoryProps> = ({ search, showChat, query, debug }) => {
  const ref = useRef<HTMLDivElement>(null);
  useSearch(ref, query);
  return (
    <MockApi
      services={() => ({ directory: new DirectoryService(), debug: new DebugService("history") })}
    >
      <StaticPlayer>
        <LayoutState showChat={showChat} />
        <StoryRouter>
          <Route
            path="/"
            element={
              <div className="layout_story__header" ref={ref}>
                <div className="layout__header">
                  {search === undefined ? <Header /> : <Header search={search} />}
                </div>
                {debug && <OpenDebugger root={ref} />}
              </div>
            }
          />
        </StoryRouter>
      </StaticPlayer>
    </MockApi>
  );
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => <HeaderStory />,
  },
  {
    name: "Chat closed",
    component: () => <HeaderStory showChat={false} />,
  },
  {
    name: "Search",
    component: () => <HeaderStory query="buddha" />,
  },
  {
    name: "No search",
    component: () => <HeaderStory search={null} />,
  },
  {
    name: "Activity monitor",
    component: () => <HeaderStory debug />,
  },
];

export default stories;
