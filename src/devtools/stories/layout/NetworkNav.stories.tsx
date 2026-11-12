// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../components/Layout/Layout.scss";

import React from "react";
import { Route } from "react-router-dom";

import NetworkNav from "../../../components/Layout/NetworkNav";
import { Provider as NetworkProvider } from "../../../contexts/Network";
import DirectoryService from "../../mocks/directory/service";
import NetworkService from "../../mocks/network/service";
import { MockApi, SessionProvider, Story, StoryRouter } from "../../storybook";

const Test: React.FC = () => (
  <MockApi
    services={() => ({ network: new NetworkService(), directory: new DirectoryService() })}
  >
    <SessionProvider>
      <NetworkProvider>
        <StoryRouter>
          <Route
            path="/*"
            element={
              <div className="layout layout--dark">
                <div style={{ display: "flex", flexDirection: "row", height: "100%" }}>
                  <NetworkNav />
                </div>
              </div>
            }
          />
        </StoryRouter>
      </NetworkProvider>
    </SessionProvider>
  </MockApi>
);

const stories: Story[] = [
  {
    name: "nav",
    component: () => <Test />,
  },
];

export default stories;
