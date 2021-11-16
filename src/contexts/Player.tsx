import React, { createContext, useMemo, useState } from "react";

import { ServiceSlug } from "../lib/directory";

export const enum PlayerMode {
  FULL,
  LARGE,
  PIP,
  CLOSED,
}

export type PlayerSource =
  | {
      type: "embed";
      service: ServiceSlug;
      id: string;
      networkKey?: string;
    }
  | {
      type: "swarm";
      swarmUri: string;
      networkKey: string;
      mimeType: string;
    };

interface PlayerValue {
  path: string;
  setPath: (path: string) => void;
  source: PlayerSource;
  setSource: (source: PlayerSource) => void;
  mode: PlayerMode;
  setMode: (mode: PlayerMode) => void;
}

export const PlayerContext = createContext<PlayerValue>(null);

export const Provider: React.FC = ({ children }) => {
  const [path, setPath] = useState<string>("");
  const [source, setSource] = useState<PlayerSource>(null);
  const [mode, setMode] = useState<PlayerMode>(PlayerMode.PIP);
  // const { theaterMode } = useLayout();

  const value = useMemo<PlayerValue>(
    () => ({
      path,
      setPath,
      source,
      setSource,
      mode,
      setMode,
    }),
    [source, mode]
  );

  // const handleClose = useCallback(() => {
  //   setMode(PlayerMode.CLOSED);
  //   setSource(null);
  // }, []);

  // const playerEmbedClass = clsx(
  //   "player_embed",
  //   theaterMode
  //     ? "player_embed--theater"
  //     : {
  //         "player_embed--full": mode === PlayerMode.FULL,
  //         "player_embed--large": mode === PlayerMode.LARGE,
  //         "player_embed--pip": mode === PlayerMode.PIP,
  //         "player_embed--closed": mode === PlayerMode.CLOSED,
  //       }
  // );

  // const embedRef = useRef<HTMLDivElement>(null);
  // const [, height] = useResizeObserver(embedRef);

  // const containerStyle: CSSProperties = {
  //   marginTop: mode === PlayerMode.LARGE ? `${height}px` : 0,
  // };

  // const scrollbarRef = useRef<Scrollbars>(null);
  // useEffect(() => {
  //   scrollbarRef.current.scrollToTop();
  // }, [theaterMode]);

  return <PlayerContext.Provider value={value}>{children}</PlayerContext.Provider>;
};

Provider.displayName = "Player.Provider";
