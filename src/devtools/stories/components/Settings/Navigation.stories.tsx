// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Settings.stories.scss";

import React, { ReactNode, useEffect, useRef, useState } from "react";
import { Route } from "react-router";

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

// The links in it, the first focused (a link needs an href to take focus),
// with each one's href below them.
const Hrefs: React.FC<{ children: ReactNode }> = ({ children }) => {
  const ref = useRef<HTMLDivElement>(null);
  const [hrefs, setHrefs] = useState<string[]>([]);
  useEffect(() => {
    const links = Array.from(ref.current.querySelectorAll("a"));
    links[0]?.focus();
    setHrefs(links.map((a) => `${a.textContent.trim()}: ${a.getAttribute("href") ?? "no href"}`));
  }, []);
  return (
    <>
      <div ref={ref} className="settings_story__row">
        {children}
      </div>
      <pre className="settings_story__state">{hrefs.join("\n")}</pre>
    </>
  );
};

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
    // Each link is a real one: it has an href (here the storybook's router's)
    // and takes focus.
    name: "Hrefs",
    component: () => (
      <InContext>
        <Hrefs>
          <Link className="settings_story__link" to="/settings/networks/1">
            Test Network
          </Link>
          <Link className="settings_story__link" to="/settings" back>
            Back to settings
          </Link>
        </Hrefs>
      </InContext>
    ),
  },
  {
    name: "Router hrefs",
    component: () => (
      <InRouter>
        <Hrefs>
          <Link className="settings_story__link" to="/settings/networks/1">
            Test Network
          </Link>
        </Hrefs>
      </InRouter>
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
