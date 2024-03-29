// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import qs from "qs";

import { Listing } from "../apis/strims/network/v1/directory/directory";
import { PlayerSource } from "../contexts/Player";

export type ServiceSlug = "angelthump" | "twitch" | "twitch-vod" | "youtube";

const serviceSlugs: [ServiceSlug, Listing.Embed.Service][] = [
  ["angelthump", Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP],
  ["twitch", Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM],
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
      const search = qs.stringify({
        k: networkKey,
        ...Object.fromEntries(content.embed.queryParams),
      });
      return `/embed/${serviceToSlug(content.embed.service)}/${content.embed.id}?${search}`;
    }
    case Listing.ContentCase.MEDIA: {
      const { mimeType, swarmUri } = content.media;
      return `/player/${networkKey}?${qs.stringify({ mimeType, swarmUri })}`;
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
    case Listing.ContentCase.MEDIA:
      return {
        type: "swarm",
        mimeType: content.media.mimeType,
        swarmUri: content.media.swarmUri,
        networkKey,
      };
    default:
      return null;
  }
};

const EMBED_ID = "([\\w-]{1,30})";
const EMBED_URLS = [
  {
    pattern: new RegExp(`twitch\\.tv/videos/${EMBED_ID}(?:.*&t=([^&$]+))`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_VOD,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
  {
    pattern: new RegExp(`twitch\\.tv/${EMBED_ID}/?$`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_TWITCH_STREAM,
      id: v[1],
    }),
  },
  {
    pattern: new RegExp(`angelthump\\.com/(?:embed/)?${EMBED_ID}$`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP,
      id: v[1],
    }),
  },
  {
    pattern: new RegExp(`player\\.angelthump\\.com/.*?[&?]channel=${EMBED_ID}`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_ANGELTHUMP,
      id: v[1],
    }),
  },
  {
    pattern: new RegExp(`youtube\\.com/watch.*?[&?]v=${EMBED_ID}(?:.*&t=([^&$]+))?`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
  {
    pattern: new RegExp(`youtu\\.be/${EMBED_ID}(?:.*[?&]t=([^&$]+))?`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
  {
    pattern: new RegExp(`youtube\\.com/embed/${EMBED_ID}(?:.*[?&]t=([^&$]+))?`),
    embed: (v: RegExpExecArray) => ({
      service: Listing.Embed.Service.DIRECTORY_LISTING_EMBED_SERVICE_YOUTUBE,
      id: v[1],
      queryParams: v[2] ? { t: v[2] } : {},
    }),
  },
];

export const createEmbedFromURL = (url: string): Listing.Embed => {
  for (const { pattern, embed } of EMBED_URLS) {
    const match = pattern.exec(url);
    if (match) {
      return new Listing.Embed(embed(match));
    }
  }
};

const fnv1a = (input: string) => {
  let hash = 2166136261;
  for (let i = 0, len = input.length; i < len; i++) {
    hash ^= input.charCodeAt(i);
    hash += (hash << 1) + (hash << 4) + (hash << 7) + (hash << 8) + (hash << 24);
  }
  return hash >>> 0;
};

const createRng = (seed: string) => {
  let n = 0;
  return () => fnv1a(`${n++}${seed}`) / 0xffffffff;
};

const generateColor = (rng: () => number) => {
  const h = Math.round(rng() * 360);
  const s = Math.round(rng() * 80 + 20);
  const l = Math.round(rng() * 50 + 20);
  return `hsl(${h}, ${s}%, ${l}%)`;
};

export const createListingRng = (listing: Listing) => {
  switch (listing?.content.case) {
    case Listing.ContentCase.MEDIA: {
      let seed: string;
      try {
        const url = new URL(listing.content.media.swarmUri);
        seed = url.searchParams.get("xt").split(":").pop();
      } catch {
        seed = listing.content.media.swarmUri;
      }
      return createRng(seed);
    }
    case Listing.ContentCase.EMBED: {
      const { id, service } = listing.content.embed;
      return createRng(`${id}:${service}`);
    }
    default:
      return createRng("default");
  }
};

export const getListingColor = (listing: Listing) => generateColor(createListingRng(listing));
