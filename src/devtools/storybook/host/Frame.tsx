// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../styles/storybook.scss";

import i18n from "i18next";
import React, { useEffect, useLayoutEffect, useMemo, useState, useSyncExternalStore } from "react";
import { createRoot } from "react-dom/client";
import { MemoryRouter } from "react-router";

import { Provider as BackgroundRouteProvider } from "../../../contexts/BackgroundRoute";
import { Provider as ThemeProvider, useTheme } from "../../../contexts/Theme";
import { StoryHostContext, StoryHostValue } from "../context";
import { StoryJson, jsonScript } from "./ErrorList";
import { StoryView } from "./StoryView";
import { IndexData, buildIndex, matchRoute } from "./catalog";
import { forgetReportedKeys, reportMissingKeys } from "./missingKeys";
import { FrameDocument, parseFrameDocument, resetFrameOrientation } from "./preload";
import { Theme, postToPage } from "./protocol";
import { Session, activate } from "./recorder";

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

// One story, the document's only one at a time: the storybook page loads a
// fresh document per story, so its modules' state, styles and the device
// type are its own; a document can also switch to another story in place
// (storyFrame).
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

// hack/storybook/check.py --compare's capture clock, in a realm it shoots
// (its CAPTURE_SCRIPT): reset puts it back at its start, its timers dropped.
const CAPTURE_CLOCK = "__storybookCapture";
interface CaptureClock {
  reset?: () => void;
}

/**
 * What a story frame document offers hack/storybook/check.py: show(hash)
 * switches to the story at the frame route `hash` (#/storybook-frame/...)
 * in place when its theme and device are the document's, else returns
 * false and does nothing.
 */
export interface FrameSwitch {
  show: (hash: string) => boolean;
}

type FrameRealm = Window & { [CAPTURE_CLOCK]?: CaptureClock; __storybookFrame?: FrameSwitch };

// Records the errors of the story `doc` names into a session of its own,
// from before its first render, so errors made while it loads and renders
// count (a translation key en lacks among them), and renders it into
// `container`; the returned function unmounts it.
const mountStory = (container: HTMLElement, doc: StoryDocument): (() => void) => {
  const session = new Session(doc.route);
  const deactivate = activate(session);
  reportMissingKeys();
  const root = createRoot(container);
  const render = () => root.render(<StoryFrame doc={doc} session={session} />);
  // Shot by hack/storybook/check.py --compare (its capture clock is in the
  // realm), the story renders once i18next has its strings, so a <Trans>
  // shows the same text whichever of them and the story's chunks comes
  // first.
  const waits = CAPTURE_CLOCK in window && !i18n.isInitialized;
  if (waits) {
    i18n.on("initialized", render);
  } else {
    render();
  }
  return () => {
    if (waits) {
      i18n.off("initialized", render);
    }
    root.unmount();
    deactivate();
  };
};

type Attributes = [string, string][];

const attributesOf = (el: Element): Attributes =>
  Array.from(el.attributes, (a) => [a.name, a.value]);

const restoreAttributes = (el: Element, saved: Attributes) => {
  const names = new Set(saved.map(([name]) => name));
  for (const { name } of Array.from(el.attributes)) {
    if (!names.has(name)) {
      el.removeAttribute(name);
    }
  }
  for (const [name, value] of saved) {
    if (el.getAttribute(name) !== value) {
      el.setAttribute(name, value);
    }
  }
};

// A story frame document: its story, and the switch to another in place
// (FrameSwitch, window.__storybookFrame). A switch unmounts the story and
// puts back what the harness owns as the document had it before its first
// story rendered: the capture clock at its start (its timers dropped,
// random and crypto reseeded), empty local and session storage, the body's
// own children alone (a portal, or check.py's masks, removed), the
// attributes of <html>, <body> and the root's container (classes, inline
// styles) in a new container, the title, the scroll, no focus or
// selection, the device's own orientation and no translation key reported
// yet; then the next story mounts on a new error-recording Session. What
// the stories' modules hold (their state, the styles they inserted,
// listeners left on the window) stays the realm's. A change of the hash
// route switches the same way, and reloads the document for another theme
// or device.
const storyFrame = (first: HTMLElement, doc: StoryDocument) => {
  let container = first;
  const baseline = {
    html: attributesOf(document.documentElement),
    body: attributesOf(document.body),
    container: attributesOf(container),
    title: document.title,
  };
  const children = new Set(Array.from(document.body.children));
  let current = doc;
  let unmount = mountStory(container, doc);

  const reset = () => {
    (window as FrameRealm)[CAPTURE_CLOCK]?.reset?.();
    try {
      localStorage.clear();
      sessionStorage.clear();
    } catch {
      // A realm without storage has none to clear.
    }
    (document.activeElement as HTMLElement | null)?.blur?.();
    document.getSelection()?.removeAllRanges();
    const fresh = document.createElement(container.tagName);
    for (const [name, value] of baseline.container) {
      fresh.setAttribute(name, value);
    }
    container.replaceWith(fresh);
    children.delete(container);
    children.add(fresh);
    container = fresh;
    for (const el of Array.from(document.body.children)) {
      if (!children.has(el)) {
        el.remove();
      }
    }
    restoreAttributes(document.documentElement, baseline.html);
    restoreAttributes(document.body, baseline.body);
    document.title = baseline.title;
    window.scrollTo(0, 0);
    document.scrollingElement?.scrollTo(0, 0);
    resetFrameOrientation();
    forgetReportedKeys();
  };

  const show = (hash: string): boolean => {
    const next = parseFrameDocument(hash);
    if (next?.kind !== "story" || next.theme !== current.theme || next.device !== current.device) {
      return false;
    }
    unmount();
    reset();
    if (location.hash !== hash) {
      history.replaceState(history.state, "", hash);
    }
    current = next;
    unmount = mountStory(container, next);
    return true;
  };
  (window as FrameRealm).__storybookFrame = { show };
  window.addEventListener("hashchange", () => {
    if (!show(location.hash)) {
      location.reload();
    }
  });
};

// Renders a storybook frame document into `root`.
export const renderFrame = (root: HTMLElement, doc: FrameDocument): void => {
  if (doc.kind === "index") {
    createRoot(root).render(<IndexFrame />);
    return;
  }
  storyFrame(root, doc);
};
