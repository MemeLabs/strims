import React, {
  ComponentType,
  RefCallback,
  RefObject,
  useCallback,
  useContext,
  useMemo,
  useRef,
  useState,
} from "react";
import { useToggle } from "react-use";

type ToggleFunc = (v?: boolean | ((v: boolean) => boolean)) => void;

export interface OverlayState {
  open: boolean;
  transitioning: boolean;
}

export interface LayoutContextProps {
  root: RefObject<HTMLElement>;
  swapMainPanels: boolean;
  overlayState: OverlayState;
  showChat: boolean;
  showVideo: boolean;
  theaterMode: boolean;
  expandNav: boolean;
  modalOpen: boolean;
  toggleSwapMainPanels: ToggleFunc;
  setOverlayState: React.Dispatch<React.SetStateAction<OverlayState>>;
  toggleOverlayOpen: ToggleFunc;
  toggleShowChat: ToggleFunc;
  toggleShowVideo: ToggleFunc;
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
    const root = useRef<HTMLElement>(null);
    const [swapMainPanels, toggleSwapMainPanels] = useToggle(false);
    const [overlayState, setOverlayState] = useState<OverlayState>({
      open: false,
      transitioning: false,
    });
    const [showChat, toggleShowChat] = useToggle(true);
    const [showVideo, toggleShowVideo] = useToggle(false);
    const [theaterMode, toggleTheaterMode] = useToggle(false);
    const [expandNav, toggleExpandNav] = useToggle(false);
    const [modalOpen, toggleModalOpen] = useToggle(false);

    const setRoot: RefCallback<HTMLElement> = useCallback((e) => (root.current = e), []);

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
        theaterMode,
        expandNav,
        modalOpen,
        toggleSwapMainPanels,
        setOverlayState,
        toggleOverlayOpen,
        toggleShowChat,
        toggleShowVideo,
        toggleTheaterMode,
        toggleExpandNav,
        toggleModalOpen,
      }),
      [swapMainPanels, overlayState, showChat, showVideo, theaterMode, expandNav, modalOpen]
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
