import React, { Suspense } from "react";

import { Provider as BackgroundRouteProvider } from "../contexts/BackgroundRoute";
import { ConnFactoryThing, Provider as SessionProvider } from "../contexts/Session";
import { Provider as ThemeProvider } from "../contexts/Theme";
import LoadingPlaceholder from "./LoadingPlaceholder";

export interface ProviderProps {
  thing: ConnFactoryThing;
}

const Provider: React.FC<ProviderProps> = ({ thing, children }) => (
  <SessionProvider thing={thing}>
    <ThemeProvider>
      <BackgroundRouteProvider>
        <Suspense fallback={<LoadingPlaceholder />}>{children}</Suspense>
      </BackgroundRouteProvider>
    </ThemeProvider>
  </SessionProvider>
);

export default Provider;
