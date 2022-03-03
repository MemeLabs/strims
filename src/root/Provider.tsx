import React, { Suspense } from "react";

import LoadingPlaceholder from "../components/LoadingPlaceholder";
import { Provider as BackgroundRouteProvider } from "../contexts/BackgroundRoute";
import { APIDialer, Provider as SessionProvider } from "../contexts/Session";
import { Provider as ThemeProvider } from "../contexts/Theme";

export interface ProviderProps {
  apiDialer: APIDialer;
}

const Provider: React.FC<ProviderProps> = ({ apiDialer, children }) => (
  <SessionProvider apiDialer={apiDialer}>
    <ThemeProvider>
      <BackgroundRouteProvider>
        <Suspense fallback={<LoadingPlaceholder />}>{children}</Suspense>
      </BackgroundRouteProvider>
    </ThemeProvider>
  </SessionProvider>
);

export default Provider;
