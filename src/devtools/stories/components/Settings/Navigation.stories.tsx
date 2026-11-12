// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Settings.stories.scss";

import React, { ReactNode } from "react";
import { Route } from "react-router-dom";

import { Link, Navigate } from "../../../../components/Settings/Navigation";
import { Provider, useNavigation } from "../../../../contexts/SettingsNavigation";
import { Story, StoryRouter } from "../../../storybook";

// The navigation context's history, its focused entry marked.
const State: React.FC = () => {
  const { history, focusedIndex, location } = useNavigation();
  return (
    <pre className="settings_story__state">
      {`location: ${location?.pathname}\nhistory:\n`}
      {history
        .map(({ pathname }, i) => `${i === focusedIndex ? "> " : "  "}${i} ${pathname}`)
        .join("\n")}
    </pre>
  );
};

const InContext: React.FC<{ initialPath?: string; children: ReactNode }> = ({
  initialPath = "/settings/networks",
  children,
}) => (
  <div className="settings_story">
    <Provider initialPath={initialPath}>
      {children}
      <State />
    </Provider>
  </div>
);

// Outside a navigation context, Link and Navigate use the router's.
const InRouter: React.FC<{ children: ReactNode }> = ({ children }) => (
  <StoryRouter initialPath="/settings/networks">
    <Route path="/settings/networks" element={<div className="settings_story">{children}</div>} />
  </StoryRouter>
);

const stories: Story[] = [
  {
    name: "Links",
    component: () => (
      <InContext>
        <div className="settings_story__row">
          <Link className="settings_story__link" to="/settings/networks/1">
            Test Network
          </Link>
          <Link className="settings_story__link" to="/settings/networks/2" replace>
            Strims (replace)
          </Link>
          <Link className="settings_story__link" to="/settings" back>
            Back to settings
          </Link>
        </div>
      </InContext>
    ),
  },
  {
    name: "Navigate",
    component: () => (
      <InContext>
        <Navigate to="/settings/networks/1" />
      </InContext>
    ),
  },
  {
    name: "Navigate replace",
    component: () => (
      <InContext>
        <Navigate to="/settings/networks/1" replace />
      </InContext>
    ),
  },
  {
    name: "Router link",
    component: () => (
      <InRouter>
        <div className="settings_story__row">
          <Link className="settings_story__link" to="/settings/networks/1">
            Test Network
          </Link>
        </div>
      </InRouter>
    ),
  },
  {
    name: "Router navigate",
    component: () => (
      <InRouter>
        <Navigate to="/settings/networks/1" />
      </InRouter>
    ),
  },
];

export default stories;
