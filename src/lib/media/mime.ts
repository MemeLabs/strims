// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** The unit format's MIME type essence. */
export const UNITS_MIME_TYPE = "video/x.strims-units";

/** How a stream's listing MIME type says its swarm is read and played. */
export const enum StreamFormat {
  /** A record per access unit, remuxed to fMP4 in the player worker. */
  Units = "units",
  Unknown = "unknown",
}

// The type and subtype of a MIME type, lowercased, without parameters.
export const mimeEssence = (mime: string): string =>
  (mime ?? "").split(";", 1)[0].trim().toLowerCase();

export const streamFormat = (mime: string): StreamFormat =>
  mimeEssence(mime) === UNITS_MIME_TYPE ? StreamFormat.Units : StreamFormat.Unknown;
