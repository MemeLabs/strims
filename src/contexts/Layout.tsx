// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ComponentType, RefCallback, useContext, useMemo, useState } from "react";

import curryDispatchActions from "../lib/curryDispatchActions";
import { DevicePreferenceStore, devicePreferences } from "../lib/devicePreferences";

type ToggleArg = boolean | ((v: boolean) => boolean);

type ToggleFunc = (v?: ToggleArg) => void;

export interface OverlayState {
  open: boolean;
  transitioning: boolean;
}

export interface LayoutState {
  swapMainPanels: boolean;
  overlayState: OverlayState;
  showChat: boolean;
  showVideo: boolean;
  fullScreenChat: boolean;
  theaterMode: boolean;
  expandNav: boolean;
  modalOpen: boolean;
}

export const initialLayoutState: LayoutState = {
  swapMainPanels: false,
  overlayState: {
    open: false,
    transitioning: false,
  },
  showChat: true,
  showVideo: false,
  fullScreenChat: false,
  theaterMode: false,
  expandNav: false,
  modalOpen: false,
};

export interface LayoutContextProps extends LayoutState {
  root: HTMLElement;
  toggleSwapMainPanels: ToggleFunc;
  setOverlayState: React.Dispatch<React.SetStateAction<OverlayState>>;
  toggleOverlayOpen: (open: boolean) => void;
  toggleShowChat: ToggleFunc;
  toggleShowVideo: ToggleFunc;
  toggleFullScreenChat: ToggleFunc;
  toggleTheaterMode: ToggleFunc;
  toggleExpandNav: ToggleFunc;
  toggleModalOpen: ToggleFunc;
}

const LayoutContext = React.createContext<LayoutContextProps>(null);

// A flag after a toggle given `v`, as react-use's useToggle has it: a
// boolean sets it, anything else (none, a click event) flips it.
const toggled = (value: boolean, v?: unknown): boolean => (typeof v === "boolean" ? v : !value);

const toggle =
  (key: keyof Omit<LayoutState, "overlayState">) =>
  (state: LayoutState, v?: ToggleArg): LayoutState => ({
    ...state,
    [key]: toggled(state[key], v),
  });

// The layout's reducers. The swap is the device's preference, so its toggle
// also stores it in `preferences`.
export const createLayoutActions = (preferences: DevicePreferenceStore) => {
  const toggleSwapMainPanels = (state: LayoutState, v?: ToggleArg): LayoutState => {
    const swapMainPanels = toggled(state.swapMainPanels, v);
    preferences.save({ swapMainPanels });
    return { ...state, swapMainPanels };
  };

  const setOverlayState = (
    state: LayoutState,
    v: React.SetStateAction<OverlayState>
  ): LayoutState => ({
    ...state,
    overlayState: typeof v === "function" ? v(state.overlayState) : v,
  });

  const toggleOverlayOpen = (state: LayoutState, open: boolean): LayoutState => ({
    ...state,
    overlayState: { open, transitioning: false },
  });

  return {
    toggleSwapMainPanels,
    setOverlayState,
    toggleOverlayOpen,
    toggleShowChat: toggle("showChat"),
    toggleShowVideo: toggle("showVideo"),
    toggleFullScreenChat: toggle("fullScreenChat"),
    toggleTheaterMode: toggle("theaterMode"),
    toggleExpandNav: toggle("expandNav"),
    toggleModalOpen: toggle("modalOpen"),
  };
};

export interface WithRootRefProps {
  rootRef: RefCallback<HTMLElement>;
}

export const withLayoutContext = <T,>(
  C: ComponentType<T & WithRootRefProps>
): React.FC<Omit<T, keyof WithRootRefProps>> => {
  const Provider: React.FC<T> = (props) => {
    const [root, setRoot] = useState<HTMLElement>(null);
    const [state, setState] = useState<LayoutState>(() => ({
      ...initialLayoutState,
      ...devicePreferences.load(),
    }));
    const actions = useMemo(
      () => curryDispatchActions(setState, createLayoutActions(devicePreferences)),
      []
    );

    const value = useMemo<LayoutContextProps>(
      () => ({ root, ...state, ...actions }),
      [root, state]
    );

    return (
      <LayoutContext.Provider value={value}>
        <C {...props} rootRef={setRoot} />
      </LayoutContext.Provider>
    );
  };

  Provider.displayName = "Layout.Provider";

  return Provider;
};

export const useLayout = (): LayoutContextProps => useContext(LayoutContext);
