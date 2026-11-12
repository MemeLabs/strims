// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { Suspense } from "react";
import { BrowserRouter, Route, Routes } from "react-router";

import { APIDialer, Provider as SessionProvider } from "../contexts/Session";
import PlayerPage from "./Player";
import Status from "./Status";
import Tiles from "./Tiles";
import { BootstrapListingsProvider } from "./listings";
import { useAnonymousSession } from "./session";

// The node in the page signs in as the page loads, so a stream opened from
// the tiles starts without waiting on it.
const WatchRoutes: React.FC = () => {
  const session = useAnonymousSession();
  return (
    <Routes>
      <Route index element={<Tiles />} />
      <Route path=":networkKey" element={<PlayerPage session={session} />} />
    </Routes>
  );
};

interface AppProps {
  apiDialer: APIDialer;
}

const App: React.FC<AppProps> = ({ apiDialer }) => (
  <BrowserRouter basename="/watch">
    <SessionProvider apiDialer={apiDialer}>
      <BootstrapListingsProvider>
        <div className="watch theme theme--dark">
          <Suspense fallback={<Status />}>
            <WatchRoutes />
          </Suspense>
        </div>
      </BootstrapListingsProvider>
    </SessionProvider>
  </BrowserRouter>
);

export default App;
