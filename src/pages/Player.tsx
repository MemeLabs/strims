import React, { useContext, useEffect } from "react";
import { useLocation, useParams } from "react-router-dom";

import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import useQuery from "../hooks/useQuery";

interface PlayerQueryParams {
  swarmUri: string;
  mimeType: string;
  listingId: string;
}

const Player: React.FC = () => {
  const params = useParams<"networkKey">();
  const location = useLocation();
  const query = useQuery<PlayerQueryParams>(location.search);
  const { toggleShowVideo, toggleOverlayOpen } = useLayout();

  const { setMode, setSource, setPath } = useContext(PlayerContext);
  useEffect(() => {
    toggleOverlayOpen(true);
    toggleShowVideo(true);
    setMode(PlayerMode.LARGE);
    setSource({
      type: "swarm",
      networkKey: params.networkKey,
      swarmUri: query.swarmUri,
      mimeType: query.mimeType,
      listingId: BigInt(query.listingId),
    });
    setPath(location.pathname + location.search);
    return () => {
      toggleOverlayOpen(false);
      setMode(PlayerMode.PIP);
    };
  }, [params.networkKey, query.swarmUri, query.mimeType, query.listingId]);

  // TODO: stream metadata - title, description, links, viewers, metrics,
  // schedule, etc...
  // directory api
  return <div style={{ height: "1000px" }} />;
};

export default Player;
