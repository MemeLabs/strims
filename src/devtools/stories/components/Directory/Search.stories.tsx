// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Directory.stories.scss";

import React, { ComponentProps, useEffect, useRef } from "react";
import { Route } from "react-router";

import Search from "../../../../components/Directory/Search";
import { DEVICE_TYPE, DeviceType } from "../../../../lib/userAgent";
import DirectoryService from "../../../mocks/directory/service";
import { MockApi, StaticPlayer, Story, StoryRouter } from "../../../storybook";

// Focuses a React-controlled input and types `text` into it, as a viewer
// would. A page without focus (a background tab, a headless browser) moves
// the focus without a focus event: React hears one anyway.
const typeInto = (input: HTMLInputElement, text: string) => {
  input.focus();
  if (!document.hasFocus()) {
    input.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
  }
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, text);
  input.dispatchEvent(new Event("input", { bubbles: true }));
};

interface SearchStoryProps extends ComponentProps<typeof Search> {
  /** Focuses the input (which loads the listings) and types this. */
  query?: string;
}

// The props the footer's search modal gives the search.
const FOOTER_MODAL = { menuOpen: true, scrollMenu: true, showCancel: true };

// The search as the layout has it: on the desktop in the header bar, on
// phones filling the footer's search modal, its menu open and a cancel
// button beside it. Selecting a result sets the player's source and
// navigates, both inside the story.
const SearchStory: React.FC<SearchStoryProps> = ({ query, ...props }) => {
  const inputRef = useRef<HTMLInputElement>(null);
  useEffect(() => {
    if (query !== undefined) {
      typeInto(inputRef.current, query);
    }
  }, []);
  const portable = DEVICE_TYPE === DeviceType.Portable;

  return (
    <MockApi services={() => ({ directory: new DirectoryService() })}>
      <StaticPlayer>
        <StoryRouter>
          <Route
            path="/"
            element={
              <div className={portable ? "directory_story__modal" : "directory_story__header"}>
                <Search inputRef={inputRef} {...(portable ? FOOTER_MODAL : {})} {...props} />
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
    component: () => <SearchStory maxResults={10} />,
  },
  {
    name: "Results",
    component: () => <SearchStory maxResults={10} query="buddha" />,
  },
  {
    name: "Many results",
    component: () => <SearchStory maxResults={10} query="e" />,
  },
  {
    name: "No results",
    component: () => <SearchStory maxResults={10} query="no such channel" menuOpen />,
  },
  {
    name: "Embed URL",
    component: () => <SearchStory maxResults={10} query="https://www.twitch.tv/somechannel" />,
  },
];

export default stories;
