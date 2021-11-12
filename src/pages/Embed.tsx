import React, { useContext, useEffect } from "react";
import { useLocation, useParams } from "react-router-dom";

import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";

interface EmbedRouteParams {
  service: string;
  id: string;
}

const Embed: React.FC = () => {
  const params = useParams<EmbedRouteParams>();
  const location = useLocation();
  const { toggleShowVideo, setShowContent } = useLayout();

  const { setMode, setSource, setPath } = useContext(PlayerContext);
  useEffect(() => {
    setShowContent({
      closed: false,
      closing: true,
      dragging: false,
    });
    toggleShowVideo(true);
    setMode(PlayerMode.FULL);
    setSource({
      type: "embed",
      service: params.service,
      id: params.id,
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
  }, [params.service, params.id]);

  return null;
};

export default Embed;
