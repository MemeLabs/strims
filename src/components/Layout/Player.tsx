import "./Player.scss";

import clsx from "clsx";
import React, { useCallback, useContext, useEffect, useRef } from "react";
import { Scrollbars } from "react-custom-scrollbars-2";
import { BsBoxArrowUpRight } from "react-icons/bs";
import { FiX } from "react-icons/fi";
import { Link } from "react-router-dom";
import { useResizeObserver } from "use-events";

import { useLayout } from "../../contexts/Layout";
import { PlayerContext, PlayerMode } from "../../contexts/Player";
import EmbedPlayer from "../EmbedPlayer";
import VideoPlayer from "../VideoPlayer";

const Player: React.FC = ({ children }) => {
  const { path, source, setSource, mode, setMode } = useContext(PlayerContext);
  const { theaterMode, toggleShowVideo } = useLayout();

  const handleClose = useCallback(() => {
    setMode(PlayerMode.CLOSED);
    setSource(null);
    toggleShowVideo(false);
  }, []);

  const className = clsx(
    "player_embed",
    {
      "player_embed--no_meta": !children,
    },
    theaterMode
      ? "player_embed--theater"
      : {
          "player_embed--full": mode === PlayerMode.FULL,
          "player_embed--large": mode === PlayerMode.LARGE,
          "player_embed--pip": mode === PlayerMode.PIP,
          "player_embed--closed": mode === PlayerMode.CLOSED,
        }
  );

  const embedRef = useRef<HTMLDivElement>(null);
  const [, height] = useResizeObserver(embedRef);

  const scrollbarRef = useRef<Scrollbars>(null);
  useEffect(() => {
    scrollbarRef.current.scrollToTop();
  }, [theaterMode]);

  return (
    <Scrollbars
      ref={scrollbarRef}
      autoHide={false}
      className={className}
      style={{ "--video-height": `${height}px`, "height": undefined }}
      renderView={(props) => <div className="player_embed__view" {...props} />}
    >
      {children && <div className="player_embed__meta">{children}</div>}
      <div className="player_embed__media" ref={embedRef}>
        {source?.type === "swarm" && (
          <VideoPlayer
            networkKey={source.networkKey}
            swarmUri={source.swarmUri}
            mimeType={source.mimeType}
            disableControls={mode === PlayerMode.PIP}
            handleClose={handleClose}
          />
        )}
        {source?.type === "embed" && (
          <EmbedPlayer
            networkKey={source.networkKey}
            service={source.service}
            id={source.id}
            queryParams={source.queryParams}
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
  );
};

export default Player;
