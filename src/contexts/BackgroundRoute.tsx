// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { createContext, useCallback, useContext, useMemo, useState } from "react";
import { Location, useLocation, useNavigate } from "react-router";

export interface State {
  location: Location;
  enabled: boolean;
}

const initialState: State = {
  location: null,
  enabled: false,
};

export interface BackgroundRouteContextValue extends State {
  toggle: (enabled: boolean) => void;
}

const BackgroundRouteContext = createContext<BackgroundRouteContextValue>(null);

export const Provider: React.FC = ({ children }) => {
  const [state, setState] = useState<State>(initialState);
  const location = useLocation();
  const navigate = useNavigate();

  const toggleModalOpen = useCallback(
    (enabled: boolean) =>
      setState((prev) => {
        if (prev.enabled && !enabled) {
          navigate(prev.location);
        }

        return {
          location: enabled ? location : null,
          enabled,
        };
      }),
    [location]
  );

  const value = useMemo<BackgroundRouteContextValue>(
    () => ({
      ...state,
      location: state.location || location,
      toggle: toggleModalOpen,
    }),
    [state, location]
  );

  return (
    <BackgroundRouteContext.Provider value={value}>{children}</BackgroundRouteContext.Provider>
  );
};

Provider.displayName = "BackgroundRoute.Provider";

export const useBackgroundRoute = (): BackgroundRouteContextValue =>
  useContext(BackgroundRouteContext);

export const Consumer = BackgroundRouteContext.Consumer;
