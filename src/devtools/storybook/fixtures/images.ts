// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import events from "../../mocks/directory/events";
import images from "../../mocks/images";

/** URLs of square emote images (PNG), also the networks' icons in the directory mock. */
export const iconUrls: string[] = images;

/** URLs of the listings' thumbnails (16:9 JPEG/PNG). */
export const thumbnailUrls: string[] = events
  .map((e) => (e.snippet?.thumbnail?.sourceOneof as { url?: string })?.url)
  .filter((url) => !!url);

// An image's bytes, for messages that carry images as data (type.Image).
export const imageData = async (url: string): Promise<Uint8Array> =>
  new Uint8Array(await (await fetch(url)).arrayBuffer());
