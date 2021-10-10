import "../styles/player.scss";

import React from "react";

export interface EmbedPlayerProps {
  service: string;
  id: string;
  disableControls: boolean;
}

const getEmbedUrl = (service: string, id: string): string | undefined => {
  switch (service) {
    case "angelthump":
      return `https://player.angelthump.com/?channel=${id}`;
    case "twitch-vod":
      return `https://player.twitch.tv/?video=v${id}&parent=${location.hostname}`;
    case "twitch-stream":
      return `https://player.twitch.tv/?channel=${id}&parent=${location.hostname}`;
    case "youtube":
      return `https://www.youtube-nocookie.com/embed/${id}?autoplay=1`;
  }
};

const EmbedPlayer: React.FC<EmbedPlayerProps> = ({ service, id }) => {
  const url = getEmbedUrl(service, id);
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
