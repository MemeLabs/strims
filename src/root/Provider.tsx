import React, { Suspense } from "react";

import { FrontendClient } from "../apis/client";
import { Provider as BackgroundRouteProvider } from "../contexts/BackgroundRoute";
import { Provider as ApiProvider } from "../contexts/FrontendApi";
import { Provider as ProfileProvider } from "../contexts/Profile";
import { Provider as ThemeProvider } from "../contexts/Theme";
import LoadingPlaceholder from "./LoadingPlaceholder";

export interface ProviderProps {
  client: FrontendClient;
}

const Provider: React.FC<ProviderProps> = ({ client, children }) => (
  <ApiProvider value={client}>
    <ProfileProvider>
      <ThemeProvider>
        <BackgroundRouteProvider>
          <Suspense fallback={<LoadingPlaceholder />}>{children}</Suspense>
        </BackgroundRouteProvider>
      </ThemeProvider>
    </ProfileProvider>
  </ApiProvider>
);

export default Provider;
