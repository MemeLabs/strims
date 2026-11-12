// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import qs from "qs";
import React, { useMemo, useState } from "react";
import { Route } from "react-router";

import PlayerPage from "../../../watch/Player";
import { AnonymousSession } from "../../../watch/session";
import NetworkService from "../../mocks/network/service";
import { MockApi, Story, StoryRouter, fixtures } from "../../storybook";
import { StorySession } from "../pages/StorySession";
import { FetchRoute, FetchStub } from "../pages/fetchStub";
import { WatchRoot, swarmStream } from "./streams";

// A network the node doesn't hold (the mock holds networks 1..3).
const STREAM_NETWORK = 7;

// The bootstrap API's invitation route (GET /api/v1/invitation/:key),
// answered by `answer`.
const invitationRoute =
  (answer: (init?: RequestInit) => Promise<Response>): FetchRoute =>
  (url, init) =>
    url.pathname.startsWith("/api/v1/invitation/") ? answer(init) : null;

// An invitation the node waits on until the page gives up on it.
const pending = (init?: RequestInit) =>
  new Promise<Response>((_, reject) =>
    init?.signal?.addEventListener("abort", () =>
      reject(new DOMException("the join was abandoned", "AbortError"))
    )
  );

const status = (status: number) => () => Promise.resolve(new Response(null, { status }));

interface PlayerStoryProps {
  /** The anonymous session; default ready. */
  session?: Partial<AnonymousSession>;
  /** The stream's query; default stream 1's. */
  query?: Record<string, string>;
  /** How the bootstrap API answers an invitation; default a 503. */
  invitation?: (init?: RequestInit) => Promise<Response>;
}

// The watch page's player at /:networkKey?swarmUri=...&mimeType=...: the
// node in the page joins the stream's network first, with an invitation the
// bootstrap API vends. Retry starts the session again (Starting).
const PlayerStory: React.FC<PlayerStoryProps> = ({
  session: initial,
  query = swarmStream(1),
  invitation = status(503),
}) => {
  const [session, setSession] = useState({ ready: true, error: null, ...initial });
  const value = useMemo<AnonymousSession>(
    () => ({ ...session, retry: () => setSession({ ready: false, error: null }) }),
    [session]
  );
  const key = fixtures.keyB64(fixtures.networkKey(STREAM_NETWORK));

  return (
    <FetchStub route={invitationRoute(invitation)}>
      <MockApi services={() => ({ network: new NetworkService({ count: 3 }) })}>
        <StorySession state={{ profile: fixtures.profile() }}>
          <WatchRoot>
            <StoryRouter initialPath={`/${key}?${qs.stringify(query)}`}>
              <Route path="/:networkKey" element={<PlayerPage session={value} />} />
            </StoryRouter>
          </WatchRoot>
        </StorySession>
      </MockApi>
    </FetchStub>
  );
};

const stories: Story[] = [
  {
    // The link names no stream (Back goes to the tiles).
    name: "Not offered",
    component: () => <PlayerStory query={{}} />,
  },
  {
    // The page's anonymous profile couldn't be made.
    name: "Couldn't start",
    component: () => (
      <PlayerStory session={{ ready: false, error: new Error("the profile store refused") }} />
    ),
  },
  {
    // The anonymous profile signing in.
    name: "Starting",
    component: () => <PlayerStory session={{ ready: false }} />,
  },
  {
    // The node waiting on the bootstrap API's invitation.
    name: "Joining",
    component: () => <PlayerStory invitation={pending} />,
  },
  {
    // The bootstrap API answered 503: worth another try.
    name: "Couldn't join",
    component: () => <PlayerStory invitation={status(503)} />,
  },
  {
    // The bootstrap API answered 404: the network takes no visitors.
    name: "No visitors",
    component: () => <PlayerStory invitation={status(404)} />,
  },
];

export default stories;
