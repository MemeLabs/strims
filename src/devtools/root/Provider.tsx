import React, { ComponentType } from "react";
import { HashRouter, MemoryRouter } from "react-router-dom";

import { DevToolsClient } from "../../apis/client";
import isPWA from "../../lib/isPWA";
import { Provider as ApiProvider } from "../contexts/DevToolsApi";

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Router: ComponentType = isPWA ? MemoryRouter : HashRouter;

const Provider = ({ client, children }: { client: DevToolsClient; children: any }) => (
  <Router>
    <React.Suspense fallback={<LoadingMessage />}>
      <ApiProvider value={client}>{children}</ApiProvider>
    </React.Suspense>
  </Router>
);

export default Provider;
