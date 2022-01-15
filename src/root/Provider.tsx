import React, { Suspense } from "react";

import { FrontendClient } from "../apis/client";
import { Provider as BackgroundRouteProvider } from "../contexts/BackgroundRoute";
import { Provider as ApiProvider } from "../contexts/FrontendApi";
// import { Provider as ProfileProvider } from "../contexts/Profile";
import { ConnFactoryThing, Provider as SessionProvider } from "../contexts/Session";
import { Provider as ThemeProvider } from "../contexts/Theme";
import LoadingPlaceholder from "./LoadingPlaceholder";

export interface ProviderProps {
  // client: FrontendClient;
  thing: ConnFactoryThing;
}

const Provider: React.FC<ProviderProps> = ({ thing, children }) => (
  <SessionProvider thing={thing}>
    {/* <ProfileProvider> */}
    <ThemeProvider>
      <BackgroundRouteProvider>
        <Suspense fallback={<LoadingPlaceholder />}>{children}</Suspense>
      </BackgroundRouteProvider>
    </ThemeProvider>
    {/* </ProfileProvider> */}
  </SessionProvider>
);

export default Provider;
