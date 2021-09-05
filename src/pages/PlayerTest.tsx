import { Base64 } from "js-base64";
import React, { useContext, useEffect } from "react";
import { useLocation, useParams } from "react-router-dom";

import { PlayerContext, PlayerMode } from "../components/PlayerEmbed";
import useQuery from "../hooks/useQuery";

interface PlayerTestRouteParams {
  networkKey: string;
}

interface PlayerTestQueryParams {
  swarmUri: string;
  mimeType: string;
}

const PlayerTest: React.FC = () => {
  const params = useParams<PlayerTestRouteParams>();
  const location = useLocation();
  const query = useQuery<PlayerTestQueryParams>(location.search);

  const { setMode, setSource, setPath } = useContext(PlayerContext);
  useEffect(() => {
    setMode(PlayerMode.LARGE);
    setSource({
      networkKey: params.networkKey,
      swarmUri: query.swarmUri,
      mimeType: query.mimeType,
    });
    setPath(location.pathname + location.search);
    return () => setMode(PlayerMode.PIP);
  }, [params.networkKey, query.swarmUri, query.mimeType]);

  // TODO: stream metadata... title/description/links/viewers/stream metrics/etc
  // directory api
  return <div style={{ height: "1000px" }} />;
};

export default PlayerTest;
