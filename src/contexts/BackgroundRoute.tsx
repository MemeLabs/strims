// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, createContext, useCallback, useContext, useMemo, useState } from "react";
import { Location, useLocation, useNavigate } from "react-router";

export interface State {
  backgroundLocation: Location;
  foregroundLocation: Location;
  enabled: boolean;
}

const initialState: State = {
  backgroundLocation: null,
  foregroundLocation: null,
  enabled: false,
};

export interface BackgroundRouteContextValue extends State {
  toggle: (enabled: boolean) => void;
}

const BackgroundRouteContext = createContext<BackgroundRouteContextValue>(null);

interface ProviderProps {
  children: ReactNode;
}

export const Provider: React.FC<ProviderProps> = ({ children }) => {
  const [state, setState] = useState<State>(initialState);
  const location = useLocation();
  const navigate = useNavigate();

  const toggleModalOpen = useCallback(
    (enabled: boolean) =>
      setState((prev) => {
        if (prev.enabled && !enabled) {
          navigate(prev.backgroundLocation);
        }

        return {
          backgroundLocation: enabled ? location : null,
          foregroundLocation: location,
          enabled,
        };
      }),
    [location]
  );

  const value = useMemo<BackgroundRouteContextValue>(
    () => ({
      ...state,
      backgroundLocation: state.backgroundLocation || location,
      foregroundLocation: location,
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
