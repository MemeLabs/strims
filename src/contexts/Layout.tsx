import React, {
  ComponentType,
  RefCallback,
  RefObject,
  useCallback,
  useContext,
  useMemo,
  useRef,
} from "react";
import { useToggle } from "react-use";

type ToggleFunc = (v?: boolean | ((v: boolean) => boolean)) => void;

export interface ContentState {
  closed: boolean;
  closing: boolean;
  dragging: boolean;
}

export interface LayoutContextProps {
  root: RefObject<HTMLElement>;
  swapMainPanels: boolean;
  memeOpen: boolean;
  showChat: boolean;
  showVideo: boolean;
  theaterMode: boolean;
  expandNav: boolean;
  toggleSwapMainPanels: ToggleFunc;
  toggleMemeOpen: ToggleFunc;
  toggleShowChat: ToggleFunc;
  toggleShowVideo: ToggleFunc;
  toggleTheaterMode: ToggleFunc;
  toggleExpandNav: ToggleFunc;
}

const LayoutContext = React.createContext<LayoutContextProps>(null);

export interface WithRootRefProps {
  rootRef: RefCallback<HTMLElement>;
}

export const withLayoutContext = <T,>(
  C: ComponentType<T & WithRootRefProps>
): React.FC<Omit<T, keyof WithRootRefProps>> => {
  const Provider: React.FC<T> = (props) => {
    const root = useRef<HTMLElement>(null);
    const [swapMainPanels, toggleSwapMainPanels] = useToggle(false);
    const [memeOpen, toggleMemeOpen] = useToggle(false);
    const [showChat, toggleShowChat] = useToggle(true);
    const [showVideo, toggleShowVideo] = useToggle(false);
    const [theaterMode, toggleTheaterMode] = useToggle(false);
    const [expandNav, toggleExpandNav] = useToggle(false);

    const setRoot: RefCallback<HTMLElement> = useCallback((e) => (root.current = e), []);

    const value = useMemo(
      () => ({
        root,
        swapMainPanels,
        memeOpen,
        showChat,
        showVideo,
        theaterMode,
        expandNav,
        toggleSwapMainPanels,
        toggleMemeOpen,
        toggleShowChat,
        toggleShowVideo,
        toggleTheaterMode,
        toggleExpandNav,
      }),
      [swapMainPanels, memeOpen, showChat, showVideo, theaterMode, expandNav]
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
