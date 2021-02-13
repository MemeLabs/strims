import React from "react";
import { HashRouter } from "react-router-dom";

import { DevToolsClient } from "../../apis/client";
import { Provider as ApiProvider } from "../contexts/DevToolsApi";

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Provider = ({ client, children }: { client: DevToolsClient; children: any }) => (
  <HashRouter>
    <React.Suspense fallback={<LoadingMessage />}>
      <ApiProvider value={client}>{children}</ApiProvider>
    </React.Suspense>
  </HashRouter>
);

export default Provider;
