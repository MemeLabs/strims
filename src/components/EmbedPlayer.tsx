import { Base64 } from "js-base64";
import qs from "qs";
import React, { useEffect } from "react";

import { useClient } from "../contexts/FrontendApi";
import { ServiceSlug, slugToService } from "../lib/directory";

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

const EmbedPlayer: React.FC<EmbedPlayerProps> = ({
  service,
  id,
  queryParams,
  networkKey: networkKeyString,
}) => {
  const client = useClient();
  const url = getEmbedUrl(service, id, queryParams);

  useEffect(() => {
    if (!networkKeyString) {
      return;
    }

    const networkKey = Base64.toUint8Array(networkKeyString);
    const res = client.directory.publish({
      networkKey,
      listing: {
        content: {
          embed: {
            service: slugToService(service),
            id,
            queryParams,
          },
        },
      },
    });
    console.log(">>> the initial conditions were", service, id, networkKeyString);
    return () => {
      console.log(">>> something changed and we fucked up");
      void res.then(({ id }) => client.directory.unpublish({ networkKey, id }));
    };
  }, [service, id, networkKeyString]);

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
