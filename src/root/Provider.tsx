// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, Suspense } from "react";

import LoadingPlaceholder from "../components/LoadingPlaceholder";
import { Provider as BackgroundRouteProvider } from "../contexts/BackgroundRoute";
import { APIDialer, Provider as SessionProvider } from "../contexts/Session";
import { Provider as ThemeProvider } from "../contexts/Theme";

export interface ProviderProps {
  apiDialer: APIDialer;
  children: ReactNode;
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
