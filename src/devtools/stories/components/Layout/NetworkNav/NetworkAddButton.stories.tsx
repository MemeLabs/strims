// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../../components/Layout/Layout.scss";
import "../../../../../components/Layout/NetworkNav/NetworkNav.scss";
import "../Layout.stories.scss";

import React, { useEffect, useRef } from "react";
import { Trans } from "react-i18next";
import { FiPlus } from "react-icons/fi";
import { Route } from "react-router-dom";

import NetworkAddButton from "../../../../../components/Layout/NetworkNav/NetworkAddButton";
import NetworkService from "../../../../mocks/network/service";
import { MockApi, Story, StoryRouter } from "../../../../storybook";
import { LayoutState, ProfileSession } from "../shell";

interface AddStoryProps {
  /** Clicks the button on mount: the add network modal opens. */
  open?: boolean;
  expand?: boolean;
}

// The button as the network nav shows it, at the nav's foot. Creating a
// network adds it to the network mock and navigates to its directory, in
// the story.
const AddStory: React.FC<AddStoryProps> = ({ open, expand }) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (open) {
      ref.current.querySelector("button").click();
    }
  }, []);
  return (
    <MockApi services={() => ({ network: new NetworkService({ count: 3 }) })}>
      <ProfileSession>
        <LayoutState expandNav={expand} />
        <StoryRouter>
          <Route
            path="/"
            element={
              <div className="layout_story__nav" ref={ref}>
                <div className="layout__nav">
                  <aside
                    className={
                      expand
                        ? "network_nav network_nav--expanded"
                        : "network_nav network_nav--collapsed"
                    }
                  >
                    <NetworkAddButton className="network_nav__add">
                      <div className="network_nav__add__gem">
                        <FiPlus />
                      </div>
                      <div className="network_nav__add__text">
                        <Trans>layout.networkNav.Add network</Trans>
                      </div>
                    </NetworkAddButton>
                  </aside>
                </div>
              </div>
            }
          />
        </StoryRouter>
      </ProfileSession>
    </MockApi>
  );
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => <AddStory />,
  },
  {
    name: "Expanded",
    component: () => <AddStory expand />,
  },
  {
    name: "Open",
    component: () => <AddStory open />,
  },
];

export default stories;
