import { Listing } from "../apis/strims/network/v1/directory/directory";
import { PlayerSource } from "../contexts/Player";

export const toEmbedService = (t: Listing.Embed.Service): string => {
  switch (t) {
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP:
      return "angelthump";
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM:
      return "twitch-stream";
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD:
      return "twitch-vod";
    case Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE:
      return "youtube";
  }
};

export const formatUri = (networkKey: string, { content }: Listing): string => {
  switch (content.case) {
    case Listing.ContentCase.EMBED:
      return `/embed/${toEmbedService(content.embed.service)}/${content.embed.id}`;
    case Listing.ContentCase.MEDIA: {
      const mimeType = encodeURIComponent(content.media.mimeType);
      const swarmUri = encodeURIComponent(content.media.swarmUri);
      return `/player/${networkKey}?mimeType=${mimeType}&swarmUri=${swarmUri}`;
    }
    default:
      return "";
  }
};

export const getListingPlayerSource = (networkKey: string, { content }: Listing): PlayerSource => {
  switch (content.case) {
    case Listing.ContentCase.EMBED:
      return {
        type: "embed",
        service: toEmbedService(content.embed.service),
        id: content.embed.id,
      };
    case Listing.ContentCase.MEDIA: {
      return {
        type: "swarm",
        mimeType: content.media.mimeType,
        swarmUri: content.media.swarmUri,
        networkKey,
      };
    }
    default:
      null;
  }
};
