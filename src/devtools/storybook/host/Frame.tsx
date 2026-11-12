// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../styles/storybook.scss";

import React, { useEffect, useLayoutEffect, useMemo, useState, useSyncExternalStore } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router-dom";

import { Provider as BackgroundRouteProvider } from "../../../contexts/BackgroundRoute";
import { Provider as ThemeProvider, useTheme } from "../../../contexts/Theme";
import { StoryHostContext, StoryHostValue } from "../context";
import { IndexData, buildIndex, matchRoute } from "./catalog";
import { StoryJson, jsonScript } from "./ErrorList";
import { reportMissingKeys } from "./missingKeys";
import type { FrameDocument } from "./preload";
import { Theme, postToPage } from "./protocol";
import { Session, activate } from "./recorder";
import { StoryView } from "./StoryView";

const ThemeSync: React.FC<{ theme: Theme }> = ({ theme }) => {
  const { colorScheme, setColorScheme } = useTheme();
  useLayoutEffect(() => {
    if (colorScheme !== theme) {
      setColorScheme(theme);
    }
  }, [theme, colorScheme]);
  return null;
};

const host: StoryHostValue = {
  setRouterPath: (path) => postToPage({ storybook: "router", path }),
};

// The recorder's snapshot, in #storybook-story and posted to the page on
// each change.
const SessionReport: React.FC<{ session: Session }> = ({ session }) => {
  const snapshot = useSyncExternalStore(session.subscribe, session.getSnapshot);
  useEffect(() => postToPage({ storybook: "story", snapshot }), [snapshot]);
  return <StoryJson snapshot={snapshot} />;
};

type StoryDocument = Extract<FrameDocument, { kind: "story" }>;

// One story, the document's only one: a fresh realm per story, so its
// modules' state, styles and the device type are its own.
const StoryFrame: React.FC<{ doc: StoryDocument; session: Session }> = ({ doc, session }) => {
  const match = useMemo(() => matchRoute(doc.route), [doc.route]);
  const initial = location.hash.replace(/^#/, "");
  return (
    <ThemeProvider>
      <ThemeSync theme={doc.theme} />
      <StoryHostContext.Provider value={host}>
        <MemoryRouter initialEntries={[initial]}>
          <BackgroundRouteProvider>
            {match ? (
              <StoryView
                moduleKey={match.key}
                name={match.name}
                device={doc.device}
                session={session}
              />
            ) : (
              <p className="storybook__message">No story module at {doc.route}</p>
            )}
          </BackgroundRouteProvider>
        </MemoryRouter>
      </StoryHostContext.Provider>
      <SessionReport session={session} />
    </ThemeProvider>
  );
};

// Every story module loaded and the catalog resolved against them, in
// #storybook-index and posted to the page.
const IndexFrame: React.FC = () => {
  const [data, setData] = useState<IndexData>(null);
  const [error, setError] = useState<string>(null);
  useEffect(() => {
    buildIndex().then(
      (index) => {
        setData(index);
        postToPage({ storybook: "index", index });
      },
      (e) => {
        setError(String(e));
        postToPage({ storybook: "index-error", error: String(e) });
      }
    );
  }, []);
  if (error) {
    return <pre className="storybook_catalog__error">{error}</pre>;
  }
  if (!data) {
    return <p className="loading_message">loading the stories</p>;
  }
  return (
    <p className="storybook__message">
      {data.stories.length} stories in {data.modules.length} modules
      <script
        type="application/json"
        id="storybook-index"
        dangerouslySetInnerHTML={jsonScript(data)}
      />
    </p>
  );
};

// Renders a storybook frame document into `root`.
export const renderFrame = (root: HTMLElement, doc: FrameDocument): void => {
  if (doc.kind === "index") {
    createRoot(root).render(<IndexFrame />);
    return;
  }
  // Recording from before the first render, so errors made while the story
  // loads and renders count, a translation key en lacks among them.
  const session = new Session(doc.route);
  activate(session);
  reportMissingKeys();
  createRoot(root).render(<StoryFrame doc={doc} session={session} />);
};
