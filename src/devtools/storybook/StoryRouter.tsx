// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useEffect } from "react";
import {
  MemoryRouter,
  Route,
  Routes,
  UNSAFE_LocationContext,
  UNSAFE_RouteContext,
  useLocation,
  useNavigate,
} from "react-router-dom";

import { Provider as BackgroundRouteProvider } from "../../contexts/BackgroundRoute";
import { useStoryHost } from "./context";

// A Router refuses to render inside another while LocationContext holds a
// value (react-router 6.6's useInRouterContext), and a nested <Routes>
// matches below its parent route's pathname (RouteContext): both reset, the
// MemoryRouter's routes match its own paths from /.
const outerLocation = null as React.ContextType<typeof UNSAFE_LocationContext>;
const rootRoute: React.ContextType<typeof UNSAFE_RouteContext> = {
  outlet: null,
  matches: [],
};

const PathReporter: React.FC = () => {
  const { pathname, search } = useLocation();
  const { setRouterPath } = useStoryHost();
  useEffect(() => setRouterPath(pathname + search), [pathname, search]);
  useEffect(() => () => setRouterPath(null), []);
  return null;
};

const NavigatedAway: React.FC<{ initialPath: string }> = ({ initialPath }) => {
  const { pathname, search } = useLocation();
  const navigate = useNavigate();
  return (
    <div className="storybook_router_away">
      <p>
        navigated to <code>{pathname + search}</code>
      </p>
      <button className="input input_button" onClick={() => navigate(-1)}>
        back
      </button>
      <button className="input input_button" onClick={() => navigate(initialPath)}>
        start over
      </button>
    </div>
  );
};

export interface StoryRouterProps {
  /** Where the story starts, e.g. "/settings/networks/1". */
  initialPath?: string;
  /** <Route> elements, matched against the in-story path. */
  children: ReactNode;
}

// A MemoryRouter of the story's own under the storybook's HashRouter, with a
// BackgroundRoute provider, so useParams reads the routes given here and
// absolute navigations (`/settings/...`) stay in the story. A path none of
// the routes match shows where it went, with a way back; the current path
// shows in the storybook's toolbar.
export const StoryRouter: React.FC<StoryRouterProps> = ({ initialPath = "/", children }) => (
  <UNSAFE_LocationContext.Provider value={outerLocation}>
    <UNSAFE_RouteContext.Provider value={rootRoute}>
      <MemoryRouter initialEntries={[initialPath]}>
        <BackgroundRouteProvider>
          <PathReporter />
          <Routes>
            {children}
            <Route path="*" element={<NavigatedAway initialPath={initialPath} />} />
          </Routes>
        </BackgroundRouteProvider>
      </MemoryRouter>
    </UNSAFE_RouteContext.Provider>
  </UNSAFE_LocationContext.Provider>
);
