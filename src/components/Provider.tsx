import * as React from "react";
import { BrowserRouter } from "react-router-dom";
import App from "../components/App";
import Client from "../service/client";

import "../main.scss";

const { Suspense } = React;

const LoadingMessage = () => (<p className="loading_message">loading</p>);

const Provider = ({client}: {client: Client}) => (
  <BrowserRouter>
    <Suspense fallback={<LoadingMessage />}>
      <App client={client} />
    </Suspense>
  </BrowserRouter>
);

export default Provider;
