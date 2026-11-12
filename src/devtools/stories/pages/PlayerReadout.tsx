// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./pages.stories.scss";

import React, { ReactNode, useMemo, useState } from "react";

import { useLayout } from "../../../contexts/Layout";
import { PlayerContext, PlayerMode, PlayerSource } from "../../../contexts/Player";

type PlayerValue = React.ContextType<typeof PlayerContext>;

const modeNames: Record<PlayerMode, string> = {
  [PlayerMode.FULL]: "FULL",
  [PlayerMode.LARGE]: "LARGE",
  [PlayerMode.PIP]: "PIP",
  [PlayerMode.CLOSED]: "CLOSED",
};

const sourceText = (source: PlayerSource | null): string =>
  source
    ? JSON.stringify(
        source,
        (_: string, v: unknown): unknown =>
          v instanceof Map ? Object.fromEntries(v as Map<string, string>) : (v ?? null),
        2
      )
    : "none";

const on = (v: boolean) => (v ? "on" : "off");

interface ReadoutState {
  mode: PlayerMode;
  source: PlayerSource | null;
  path: string;
}

// A PlayerContext that keeps what the page under it asks of the player,
// shown with the layout's toggles: the player pages render nothing of their
// own (VideoTitle only sets the document's title) and the player itself is
// the layout's.
export const PlayerReadout: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [state, setState] = useState<ReadoutState>({
    mode: PlayerMode.PIP,
    source: null,
    path: "",
  });
  const setters = useMemo(
    () => ({
      setMode: (mode: PlayerMode) => setState((prev) => ({ ...prev, mode })),
      setSource: (source: PlayerSource) => setState((prev) => ({ ...prev, source })),
      setPath: (path: string) => setState((prev) => ({ ...prev, path })),
    }),
    []
  );
  const value = useMemo<PlayerValue>(
    () => ({ ...state, ...setters, networkKey: null, listingId: null }),
    [state]
  );
  const layout = useLayout();

  return (
    <PlayerContext.Provider value={value}>
      {children}
      <dl className="pages_story_player">
        <dt>player mode</dt>
        <dd>{modeNames[state.mode]}</dd>
        <dt>player path</dt>
        <dd>
          <code>{state.path || "none"}</code>
        </dd>
        <dt>source</dt>
        <dd>
          <pre>{sourceText(state.source)}</pre>
        </dd>
        <dt>layout</dt>
        <dd>
          video {on(layout.showVideo)}, overlay {on(layout.overlayState.open)}, theater mode{" "}
          {on(layout.theaterMode)} (alt+t)
        </dd>
      </dl>
    </PlayerContext.Provider>
  );
};
