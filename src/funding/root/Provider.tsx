import { PayPalScriptProvider } from "@paypal/react-paypal-js";
import * as React from "react";
import { BrowserRouter } from "react-router-dom";

import * as cfg from "../../../funding/config.json";
import { FundingClient } from "../../lib/api";
import { Provider as ApiProvider } from "../contexts/Api";

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Provider = ({ client, children }: { client: FundingClient; children: any }) => (
  <BrowserRouter>
    <React.Suspense fallback={<LoadingMessage />}>
      <PayPalScriptProvider
        options={{
          "client-id": cfg.paypal.client_id,
          vault: true,
          components: "buttons",
          intent: "subscription",
          debug: true,
        }}
      >
        <ApiProvider value={client}>{children}</ApiProvider>
      </PayPalScriptProvider>
    </React.Suspense>
  </BrowserRouter>
);

export default Provider;
