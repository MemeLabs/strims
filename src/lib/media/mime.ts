// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

/** The unit format's MIME type essence. */
export const UNITS_MIME_TYPE = "video/x.strims-units";

/** How a stream's listing MIME type says its swarm is read and played. */
export const enum StreamFormat {
  /** A record per access unit, remuxed to fMP4 in the player worker. */
  Units = "units",
  /** A record per fMP4 segment, `[u16 init length][init][media]`. */
  Fmp4 = "fmp4",
  Unknown = "unknown",
}

// The type and subtype of a MIME type, lowercased, without parameters.
export const mimeEssence = (mime: string): string =>
  (mime ?? "").split(";", 1)[0].trim().toLowerCase();

export const streamFormat = (mime: string): StreamFormat => {
  switch (mimeEssence(mime)) {
    case UNITS_MIME_TYPE:
      return StreamFormat.Units;
    case "video/mp4":
      return StreamFormat.Fmp4;
    default:
      return StreamFormat.Unknown;
  }
};

// Whether a MIME type names its codecs, as a SourceBuffer type must.
export const hasCodecs = (mime: string): boolean =>
  (mime ?? "")
    .split(";")
    .slice(1)
    .some((p) => p.trim().toLowerCase().startsWith("codecs="));
