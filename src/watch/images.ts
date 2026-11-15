// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import {
  FrontendWatchListingsResponse_Event,
  FrontendWatchListingsResponse_Event_EventCase,
  ListingSnippetImage,
  ListingSnippetImage_SourceOneof,
  ListingSnippetImage_SourceOneofCase,
} from "../apis/strims/network/v1/directory/directory.pb";

// An image the API names by path, pointed at `base`; none when its URL
// doesn't resolve or isn't http(s). Listings' publishers write these URLs,
// so one listing's bad URL leaves only its own tile without the image.
const resolveImage = (
  image: ListingSnippetImage | undefined,
  base: string
): ListingSnippetImage | undefined => {
  if (image?.sourceOneof.case !== ListingSnippetImage_SourceOneofCase.URL) return image;
  let url: URL;
  try {
    url = new URL(image.sourceOneof.url, base);
  } catch {
    return undefined;
  }
  if (url.protocol !== "https:" && url.protocol !== "http:") return undefined;
  image.sourceOneof = new ListingSnippetImage_SourceOneof({ url: url.href });
  return image;
};

/**
 * Points the images of the listings in `events` at the bootstrap API
 * (`base`, its origin), in place, dropping those that can't be.
 */
export const resolveImages = (
  events: FrontendWatchListingsResponse_Event[],
  base: string
): FrontendWatchListingsResponse_Event[] => {
  for (const { event } of events) {
    if (event.case !== FrontendWatchListingsResponse_Event_EventCase.CHANGE) continue;
    for (const { snippet } of event.change.listings?.listings ?? []) {
      if (!snippet) continue;
      snippet.thumbnail = resolveImage(snippet.thumbnail, base);
      snippet.channelLogo = resolveImage(snippet.channelLogo, base);
    }
  }
  return events;
};
