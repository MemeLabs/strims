import "../styles/player.scss";

import clsx from "clsx";
import React, { CSSProperties, createContext, useMemo, useRef, useState } from "react";
import { Scrollbars } from "react-custom-scrollbars";
import { useResizeObserver } from "use-events";

import VideoPlayer from "./VideoPlayer";

export const enum PlayerMode {
  LARGE,
  PIP,
  CLOSED,
}

interface PlayerSource {
  swarmUri: string;
  networkKey: Uint8Array;
  mimeType: string;
}

interface PlayerState {
  source: PlayerSource;
  setSource: (source: PlayerSource) => void;
  mode: PlayerMode;
  setMode: (mode: PlayerMode) => void;
}

export const PlayerContext = createContext<PlayerState>(null);

const PlayerEmbed: React.FC = ({ children }) => {
  const [source, setSource] = useState<PlayerSource>(null);
  const [mode, setMode] = useState<PlayerMode>(PlayerMode.PIP);

  const value = useMemo<PlayerState>(
    () => ({
      source,
      setSource,
      mode,
      setMode,
    }),
    [source, mode]
  );

  const playerEmbedClass = clsx({
    "player_embed": true,
    "player_embed--large": mode == PlayerMode.LARGE,
    "player_embed--pip": mode == PlayerMode.PIP,
    "player_embed--closed": mode == PlayerMode.CLOSED,
  });

  const embedRef = useRef<HTMLDivElement>(null);
  const [, height] = useResizeObserver(embedRef);

  const containerStyle: CSSProperties = {
    marginTop: mode == PlayerMode.LARGE ? `${height}px` : 0,
  };

  return (
    <PlayerContext.Provider value={value}>
      <Scrollbars>
        <div className="player_embed__container" style={containerStyle}>
          {children}
        </div>
        <div className={playerEmbedClass} ref={embedRef}>
          {source && (
            <VideoPlayer
              networkKey={source.networkKey}
              swarmUri={source.swarmUri}
              mimeType={source.mimeType}
              disableControls={mode == PlayerMode.PIP}
            />
          )}
        </div>
      </Scrollbars>
    </PlayerContext.Provider>
  );
};

export default PlayerEmbed;
