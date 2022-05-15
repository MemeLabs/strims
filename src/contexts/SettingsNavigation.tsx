// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { createContext, useCallback, useContext, useMemo, useState } from "react";
import { Location, useLocation as useRouterLocation } from "react-router-dom";

interface NavigationPushOptions {
  back?: boolean;
  replace?: boolean;
}

interface NavigationContextValue {
  history: Location[];
  focusedIndex: number;
  location?: Location;
  push(path: string, options?: NavigationPushOptions): void;
  focusPrev(): void;
  focusNext(): void;
}

const NavigationContext = createContext<NavigationContextValue>(null);

interface ProviderProps {
  initialPath: string;
}

export const Provider: React.FC<ProviderProps> = ({ children, initialPath }) => {
  const [[history, focusedIndex], setHistory] = useState<[Location[], number]>([
    [parseLocation(initialPath)],
    0,
  ]);

  const push = useCallback(
    (href: string, options?: NavigationPushOptions) =>
      setHistory(([history, focusedIndex]) => {
        let focusIndexOffset = focusedIndex;
        if (options?.back) {
          focusIndexOffset -= 2;
        } else if (options?.replace) {
          focusIndexOffset -= 1;
        }
        return [[...history.slice(0, focusIndexOffset), parseLocation(href)], focusIndexOffset + 1];
      }),
    []
  );

  const focusPrev = useCallback(
    () => setHistory(([history, focusedIndex]) => [history, Math.max(focusedIndex - 1, 0)]),
    []
  );

  const focusNext = useCallback(
    () =>
      setHistory(([history, focusedIndex]) => [
        history,
        Math.min(focusedIndex + 1, history.length - 1),
      ]),
    []
  );

  const value = useMemo<NavigationContextValue>(
    () => ({
      history,
      focusedIndex,
      location: history[focusedIndex],
      push,
      focusPrev,
      focusNext,
    }),
    [history, focusedIndex]
  );
  return <NavigationContext.Provider value={value}>{children}</NavigationContext.Provider>;
};

export const useNavigation = () => useContext(NavigationContext);

export const useInNavigationContext = () => useNavigation() !== undefined;

export const parseLocation = (href: string) => ({
  pathname: href,
  hash: "",
  search: "",
  state: null,
  key: null,
});

export const useLinkClickHandler = (to: string, options: NavigationPushOptions) => {
  const navigation = useNavigation();
  return () => void navigation.push(to, options);
};

export const useLocation = () => useNavigation()?.location ?? useRouterLocation();
