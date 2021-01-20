import * as React from "react";

import { FrontendClient } from "../apis/client";
import { Provider as ApiProvider } from "../contexts/Api";
import { Provider as ProfileProvider } from "../contexts/Profile";
import { Provider as ThemeProvider } from "../contexts/Theme";

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Provider = ({ client, children }: { client: FrontendClient; children: any }) => (
  <React.Suspense fallback={<LoadingMessage />}>
    <ApiProvider value={client}>
      <ThemeProvider>
        <ProfileProvider>{children}</ProfileProvider>
      </ThemeProvider>
    </ApiProvider>
  </React.Suspense>
);

export default Provider;
