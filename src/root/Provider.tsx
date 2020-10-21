import * as React from "react";
import { BrowserRouter } from "react-router-dom";

import { Provider as ApiProvider } from "../contexts/Api";
import { Provider as ProfileProvider } from "../contexts/Profile";
import { Provider as ThemeProvider } from "../contexts/Theme";
import Client from "../lib/api";

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Provider = ({ client, children }: { client: Client; children: any }) => (
  <BrowserRouter>
    <React.Suspense fallback={<LoadingMessage />}>
      <ApiProvider value={client}>
        <ThemeProvider>
          <ProfileProvider>{children}</ProfileProvider>
        </ThemeProvider>
      </ApiProvider>
    </React.Suspense>
  </BrowserRouter>
);

export default Provider;
