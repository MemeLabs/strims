import React, { useContext, useEffect } from "react";
import { useLocation, useParams } from "react-router-dom";

import { useLayout } from "../contexts/Layout";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import useQuery from "../hooks/useQuery";
import { ServiceSlug } from "../lib/directory";

interface EmbedRouteParams {
  service: ServiceSlug;
  id: string;
}

interface EmbedQueryParams {
  k: string;
}

const Embed: React.FC = () => {
  const routeParams = useParams<EmbedRouteParams>();
  const location = useLocation();
  const queryParams = useQuery<EmbedQueryParams>(location.search);
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
      service: routeParams.service,
      id: routeParams.id,
      networkKey: queryParams.k,
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
  }, [routeParams.service, routeParams.id, queryParams.k]);

  return null;
};

export default Embed;
