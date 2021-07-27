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
  const query = useQuery<PlayerTestQueryParams>(useLocation().search);

  const { setMode, setSource } = useContext(PlayerContext);
  useEffect(() => {
    setMode(PlayerMode.LARGE);
    setSource({
      networkKey: Base64.toUint8Array(params.networkKey),
      swarmUri: query.swarmUri,
      mimeType: query.mimeType,
    });
    return () => setMode(PlayerMode.PIP);
  }, [params.networkKey, query.swarmUri, query.mimeType]);

  // TODO: stream metadata... title/description/links/viewers/stream metrics/etc
  // directory api
  return null;
};

export default PlayerTest;
