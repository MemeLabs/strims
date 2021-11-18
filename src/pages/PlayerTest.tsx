import React, { useContext, useEffect } from "react";
import { useLocation, useParams } from "react-router-dom";

import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import useQuery from "../hooks/useQuery";

interface PlayerTestQueryParams {
  swarmUri: string;
  mimeType: string;
}

const PlayerTest: React.FC = () => {
  const params = useParams<"networkKey">();
  const location = useLocation();
  const query = useQuery<PlayerTestQueryParams>(location.search);
  const { toggleShowVideo, setShowContent } = useLayout();

  const { setMode, setSource, setPath } = useContext(PlayerContext);
  useEffect(() => {
    setShowContent({
      closed: false,
      closing: true,
      dragging: false,
    });
    toggleShowVideo(true);
    setMode(PlayerMode.LARGE);
    setSource({
      type: "swarm",
      networkKey: params.networkKey,
      swarmUri: query.swarmUri,
      mimeType: query.mimeType,
    });
    setPath(location.pathname + location.search);
    return () => {
      setShowContent({
        closed: true,
        closing: false,
        dragging: false,
      });
      setMode(PlayerMode.PIP);
    };
  }, [params.networkKey, query.swarmUri, query.mimeType]);

  // TODO: stream metadata... title/description/links/viewers/stream metrics/etc
  // directory api
  return <div style={{ height: "1000px" }} />;
};

export default PlayerTest;
