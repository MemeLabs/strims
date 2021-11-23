import { Base64 } from "js-base64";
import { isEmpty } from "lodash";
import qs from "qs";

import { Listing } from "../apis/strims/network/v1/directory/directory";
import { PlayerSource } from "../contexts/Player";

export type ServiceSlug = "angelthump" | "twitch-stream" | "twitch-vod" | "youtube";

const serviceSlugs: [ServiceSlug, Listing.Embed.Service][] = [
  ["angelthump", Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP],
  ["twitch-stream", Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM],
  ["twitch-vod", Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD],
  ["youtube", Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE],
];

export const serviceToSlug = (t: Listing.Embed.Service): ServiceSlug =>
  serviceSlugs.find(([, k]) => k === t)[0];

export const slugToService = (t: ServiceSlug): Listing.Embed.Service =>
  serviceSlugs.find(([k]) => k === t)[1];

export const formatUri = (networkKey: string, { content }: Listing): string => {
  switch (content.case) {
    case Listing.ContentCase.EMBED: {
      let search = "?k=" + networkKey;
      if (!isEmpty(content.embed.queryParams)) {
        search += "&" + qs.stringify(Object.fromEntries(content.embed.queryParams));
      }
      return `/embed/${serviceToSlug(content.embed.service)}/${content.embed.id}${search}`;
    }
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
        service: serviceToSlug(content.embed.service),
        id: content.embed.id,
        queryParams: content.embed.queryParams,
        networkKey,
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
