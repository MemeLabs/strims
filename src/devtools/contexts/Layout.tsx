import React, { RefCallback, RefObject, useCallback, useRef } from "react";
import { useToggle } from "react-use";

type ToggleFunc = (v?: boolean | ((v: boolean) => boolean)) => void;

export interface LayoutContextProps {
  root: RefObject<HTMLElement>;
  swapMainPanels: boolean; // swap
  showContent: boolean; // meme_open
  showChat: boolean;
  showVideo: boolean;
  theaterMode: boolean;
  expandNav: boolean;
  toggleSwapMainPanels: ToggleFunc;
  toggleShowContent: ToggleFunc;
  toggleShowChat: ToggleFunc;
  toggleShowVideo: ToggleFunc;
  toggleTheaterMode: ToggleFunc;
  toggleExpandNav: ToggleFunc;
}

export const LayoutContext = React.createContext<LayoutContextProps>(null);

interface LayoutContextProviderProps {
  children: React.ComponentType<{ rootRef: RefCallback<HTMLElement> }>;
}

export const LayoutContextProvider: React.FC<LayoutContextProviderProps> = ({ children: C }) => {
  const root = useRef<HTMLElement>(null);
  const [swapMainPanels, toggleSwapMainPanels] = useToggle(false);
  const [showContent, toggleShowContent] = useToggle(false);
  const [showChat, toggleShowChat] = useToggle(true);
  const [showVideo, toggleShowVideo] = useToggle(true);
  const [theaterMode, toggleTheaterMode] = useToggle(false);
  const [expandNav, toggleExpandNav] = useToggle(false);

  const setRoot: RefCallback<HTMLElement> = useCallback((e) => (root.current = e), []);

  return (
    <LayoutContext.Provider
      value={{
        root,
        swapMainPanels,
        showContent,
        showChat,
        showVideo,
        theaterMode,
        expandNav,
        toggleSwapMainPanels,
        toggleShowContent,
        toggleShowChat,
        toggleShowVideo,
        toggleTheaterMode,
        toggleExpandNav,
      }}
    >
      <C rootRef={setRoot} />
    </LayoutContext.Provider>
  );
};
