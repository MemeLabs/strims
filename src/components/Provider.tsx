import * as React from "react";
import { BrowserRouter } from "react-router-dom";
import App, { API } from "../components/App";

import "../main.scss";

const { Suspense } = React;

const LoadingMessage = () => (<p className="loading_message">loading</p>);

const Provider = ({client}: {client: API}) => (
  <BrowserRouter>
    <Suspense fallback={<LoadingMessage />}>
      <App client={client} />
    </Suspense>
  </BrowserRouter>
);

export default Provider;
