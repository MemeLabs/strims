// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ComponentType, RefCallback, useContext, useMemo, useState } from "react";
import { useToggle } from "react-use";

type ToggleFunc = (v?: boolean | ((v: boolean) => boolean)) => void;

export interface OverlayState {
  open: boolean;
  transitioning: boolean;
}

export interface LayoutContextProps {
  root: HTMLElement;
  swapMainPanels: boolean;
  overlayState: OverlayState;
  showChat: boolean;
  showVideo: boolean;
  fullScreenChat: boolean;
  theaterMode: boolean;
  expandNav: boolean;
  modalOpen: boolean;
  toggleSwapMainPanels: ToggleFunc;
  setOverlayState: React.Dispatch<React.SetStateAction<OverlayState>>;
  toggleOverlayOpen: ToggleFunc;
  toggleShowChat: ToggleFunc;
  toggleShowVideo: ToggleFunc;
  toggleFullScreenChat: ToggleFunc;
  toggleTheaterMode: ToggleFunc;
  toggleExpandNav: ToggleFunc;
  toggleModalOpen: ToggleFunc;
}

const LayoutContext = React.createContext<LayoutContextProps>(null);

export interface WithRootRefProps {
  rootRef: RefCallback<HTMLElement>;
}

export const withLayoutContext = <T,>(
  C: ComponentType<T & WithRootRefProps>
): React.FC<Omit<T, keyof WithRootRefProps>> => {
  const Provider: React.FC<T> = (props) => {
    const [root, setRoot] = useState<HTMLElement>(null);
    const [swapMainPanels, toggleSwapMainPanels] = useToggle(false);
    const [overlayState, setOverlayState] = useState<OverlayState>({
      open: false,
      transitioning: false,
    });
    const [showChat, toggleShowChat] = useToggle(true);
    const [showVideo, toggleShowVideo] = useToggle(false);
    const [fullScreenChat, toggleFullScreenChat] = useToggle(false);
    const [theaterMode, toggleTheaterMode] = useToggle(false);
    const [expandNav, toggleExpandNav] = useToggle(false);
    const [modalOpen, toggleModalOpen] = useToggle(false);

    const toggleOverlayOpen = (open: boolean) =>
      setOverlayState({
        open,
        transitioning: false,
      });

    const value = useMemo(
      () => ({
        root,
        swapMainPanels,
        overlayState,
        showChat,
        showVideo,
        fullScreenChat,
        theaterMode,
        expandNav,
        modalOpen,
        toggleSwapMainPanels,
        setOverlayState,
        toggleOverlayOpen,
        toggleShowChat,
        toggleShowVideo,
        toggleFullScreenChat,
        toggleTheaterMode,
        toggleExpandNav,
        toggleModalOpen,
      }),
      [
        root,
        swapMainPanels,
        overlayState,
        showChat,
        showVideo,
        fullScreenChat,
        theaterMode,
        expandNav,
        modalOpen,
      ]
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
