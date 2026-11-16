// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ComponentType, RefCallback, useContext, useEffect, useMemo, useState } from "react";

import curryDispatchActions from "../lib/curryDispatchActions";
import {
  DevicePreferenceStore,
  DevicePreferences,
  devicePreferences,
} from "../lib/devicePreferences";

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

// A toggle of `key`. One that leaves the flag as it is returns the state it
// was given, so React skips the render, as useToggle's did: the layout's
// toggleFullScreenChat(false) on each navigation renders nothing.
const toggle =
  (key: keyof Omit<LayoutState, "overlayState">) =>
  (state: LayoutState, v?: ToggleArg): LayoutState => {
    const value = toggled(state[key], v);
    return value === state[key] ? state : { ...state, [key]: value };
  };

// The layout's reducers, pure, as React may run an updater twice. The swap
// is the device's preference: the provider stores it once it changes
// (createPreferenceSaver).
export const createLayoutActions = () => {
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
    toggleSwapMainPanels: toggle("swapMainPanels"),
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

// Stores the device preferences a layout state holds in `store` when they
// differ from those last loaded (`loaded`) or stored, so a state seen again
// (an effect run twice) stores nothing.
export const createPreferenceSaver = (
  store: DevicePreferenceStore,
  loaded: DevicePreferences
): ((state: LayoutState) => void) => {
  let stored = loaded.swapMainPanels;
  return ({ swapMainPanels }) => {
    if (swapMainPanels !== stored) {
      stored = swapMainPanels;
      store.save({ swapMainPanels });
    }
  };
};

// Whether the user can see the layout's chat: on a portable device the
// drawer open (closed, its peek shows none of the chat), else the chat panel
// shown.
export const chatOnScreen = (
  { overlayState, showChat }: Pick<LayoutState, "overlayState" | "showChat">,
  portable: boolean
): boolean => (portable ? overlayState.open : showChat);

export interface WithRootRefProps {
  rootRef: RefCallback<HTMLElement>;
}

export const withLayoutContext = <T,>(
  C: ComponentType<T & WithRootRefProps>
): React.FC<Omit<T, keyof WithRootRefProps>> => {
  const Provider: React.FC<T> = (props) => {
    const [root, setRoot] = useState<HTMLElement>(null);
    const [loaded] = useState(() => devicePreferences.load());
    const [state, setState] = useState<LayoutState>(() => ({ ...initialLayoutState, ...loaded }));
    const actions = useMemo(() => curryDispatchActions(setState, createLayoutActions()), []);

    const savePreferences = useMemo(() => createPreferenceSaver(devicePreferences, loaded), []);
    useEffect(() => savePreferences(state), [state.swapMainPanels]);

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
