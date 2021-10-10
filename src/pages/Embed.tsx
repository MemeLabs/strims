import React, { useContext, useEffect } from "react";
import { useLocation, useParams } from "react-router-dom";

import { PlayerContext, PlayerMode } from "../components/PlayerEmbed";

interface EmbedRouteParams {
  service: string;
  id: string;
}

const Embed: React.FC = () => {
  const params = useParams<EmbedRouteParams>();
  const location = useLocation();

  const { setMode, setSource, setPath } = useContext(PlayerContext);
  useEffect(() => {
    setMode(PlayerMode.FULL);
    setSource({
      type: "embed",
      service: params.service,
      id: params.id,
    });
    setPath(location.pathname + location.search);
    return () => setMode(PlayerMode.PIP);
  }, [params.service, params.id]);

  return null;
};

export default Embed;
