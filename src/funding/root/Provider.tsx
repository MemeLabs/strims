import * as React from "react";
import { BrowserRouter } from "react-router-dom";

import { FundingClient } from "../../lib/api";
import { Provider as ApiProvider } from "../contexts/Api";

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Provider = ({ client, children }: { client: FundingClient; children: any }) => (
  <BrowserRouter>
    <React.Suspense fallback={<LoadingMessage />}>
      <ApiProvider value={client}>{children}</ApiProvider>
    </React.Suspense>
  </BrowserRouter>
);

export default Provider;