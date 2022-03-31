import "./EmbedPlayer.scss";

import qs from "qs";
import React from "react";

import { ServiceSlug } from "../lib/directory";

export interface EmbedPlayerProps {
  service: ServiceSlug;
  id: string;
  queryParams?: Map<string, string>;
  networkKey?: string;
  disableControls?: boolean;
}

const getEmbedUrl = (
  service: string,
  id: string,
  queryParams: Map<string, string> = new Map()
): string | undefined => {
  const queryString = qs.stringify(Object.fromEntries(queryParams.entries()));
  switch (service) {
    case "angelthump":
      return `https://player.angelthump.com/?channel=${id}`;
    case "twitch-vod":
      return `https://player.twitch.tv/?video=v${id}&parent=${location.hostname}${queryString}`;
    case "twitch-stream":
      return `https://player.twitch.tv/?channel=${id}&parent=${location.hostname}`;
    case "youtube":
      return `https://www.youtube-nocookie.com/embed/${id}?autoplay=1${queryString}`;
  }
};

const EmbedPlayer: React.FC<EmbedPlayerProps> = ({ service, id, queryParams }) => {
  const url = getEmbedUrl(service, id, queryParams);

  if (!url) {
    return null;
  }

  return (
    <iframe
      className="embed_player__frame"
      width="100%"
      height="100%"
      frameBorder="0"
      scrolling="no"
      seamless
      allow="autoplay; fullscreen"
      allowFullScreen
      src={url}
    />
  );
};

export default EmbedPlayer;
