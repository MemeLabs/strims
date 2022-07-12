// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode } from "react";
import { HashRouter, MemoryRouter } from "react-router-dom";

import { DevToolsClient } from "../../apis/client";
import { Provider as BackgroundRouteProvider } from "../../contexts/BackgroundRoute";
import { IS_PWA } from "../../lib/userAgent";
import { Provider as ApiProvider } from "../contexts/DevToolsApi";

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Router = IS_PWA ? MemoryRouter : HashRouter;

interface ProviderProps {
  client: DevToolsClient;
  children: ReactNode;
}

const Provider: React.FC<ProviderProps> = ({ client, children }) => (
  <Router>
    <React.Suspense fallback={<LoadingMessage />}>
      <ApiProvider value={client}>
        <BackgroundRouteProvider>{children}</BackgroundRouteProvider>
      </ApiProvider>
    </React.Suspense>
  </Router>
);

export default Provider;
