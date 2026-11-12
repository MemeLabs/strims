// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../components/Layout/Layout.scss";

import React from "react";
import { Route } from "react-router-dom";

import Header from "../../../components/Layout/Header";
import { Provider as ThemeProvider } from "../../../contexts/Theme";
import { Story, StoryRouter } from "../../storybook";

const Test: React.FC = () => (
  <ThemeProvider>
    <StoryRouter>
      <Route
        path="/*"
        element={
          <div className="layout layout--dark">
            <Header search={null} />
          </div>
        }
      />
    </StoryRouter>
  </ThemeProvider>
);

const stories: Story[] = [
  {
    name: "header",
    component: () => <Test />,
  },
];

export default stories;
