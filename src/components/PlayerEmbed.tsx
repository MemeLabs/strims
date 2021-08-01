import "../styles/player.scss";

import clsx from "clsx";
import React, {
  CSSProperties,
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { BsBoxArrowUpRight } from "react-icons/bs";
import { FiX } from "react-icons/fi";
import { Link } from "react-router-dom";
import { useResizeObserver } from "use-events";

import { MainLayoutContext } from "./MainLayout";
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
  path: string;
  setPath: (path: string) => void;
  source: PlayerSource;
  setSource: (source: PlayerSource) => void;
  mode: PlayerMode;
  setMode: (mode: PlayerMode) => void;
}

export const PlayerContext = createContext<PlayerState>(null);

const PlayerEmbed: React.FC = ({ children }) => {
  const [path, setPath] = useState<string>("");
  const [source, setSource] = useState<PlayerSource>(null);
  const [mode, setMode] = useState<PlayerMode>(PlayerMode.PIP);
  const { theaterMode } = useContext(MainLayoutContext);

  const context = useMemo<PlayerState>(
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

  const handleClose = useCallback(() => {
    setMode(PlayerMode.CLOSED);
    setSource(null);
  }, []);

  const playerEmbedClass = clsx(
    "player_embed",
    theaterMode
      ? "player_embed--theater"
      : {
          "player_embed--large": mode === PlayerMode.LARGE,
          "player_embed--pip": mode === PlayerMode.PIP,
          "player_embed--closed": mode === PlayerMode.CLOSED,
        }
  );

  const embedRef = useRef<HTMLDivElement>(null);
  const [, height] = useResizeObserver(embedRef);

  const containerStyle: CSSProperties = {
    marginTop: mode === PlayerMode.LARGE ? `${height}px` : 0,
  };

  const scrollbarRef = useRef<Scrollbars>(null);
  useEffect(() => {
    scrollbarRef.current.scrollToTop();
  }, [theaterMode]);

  return (
    <PlayerContext.Provider value={context}>
      <Scrollbars ref={scrollbarRef}>
        <div className="player_embed__container" style={containerStyle}>
          {children}
        </div>
        <div className={playerEmbedClass} ref={embedRef}>
          {source && (
            <VideoPlayer
              networkKey={source.networkKey}
              swarmUri={source.swarmUri}
              mimeType={source.mimeType}
              disableControls={mode === PlayerMode.PIP}
            />
          )}
          {mode === PlayerMode.PIP && (
            <div className="player_embed__pip_mask">
              <button className="player_embed__pip_mask__close" onClick={handleClose}>
                <FiX size={22} />
              </button>
              <Link to={path} className="player_embed__pip_mask__expand">
                <BsBoxArrowUpRight size={22} />
              </Link>
            </div>
          )}
        </div>
      </Scrollbars>
    </PlayerContext.Provider>
  );
};

export default PlayerEmbed;
