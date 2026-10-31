// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { StreamFormat } from "./mime";

/** How a stream plays. */
export const enum PlayerKind {
  /** A unit-format stream through WebCodecs, at a low lag. */
  LowLatency = "lowlatency",
  /** A unit-format stream remuxed for MediaSource: the baseline player. */
  Units = "units",
  /** A stream this UI can't play. */
  None = "none",
}

/** Which player a viewer asked for unit-format streams. */
export type PlayerPreference = "lowlatency" | "baseline";

/** The query parameter, and the local storage key, naming the preferred player. */
export const PLAYER_PARAM = "player";
export const PLAYER_STORAGE_KEY = "player";

const preference = (value: string | null | undefined): PlayerPreference | null =>
  value === "lowlatency" || value === "baseline" ? value : null;

// The player a viewer prefers: `?player=lowlatency|baseline`, else the
// `player` local storage setting, else the baseline.
export const preferredPlayer = (
  search: string,
  storage: Pick<Storage, "getItem"> | null
): PlayerPreference => {
  const param = preference(new URLSearchParams(search).get(PLAYER_PARAM));
  if (param) return param;
  let stored: string | null = null;
  try {
    stored = storage?.getItem(PLAYER_STORAGE_KEY) ?? null;
  } catch {
    // Storage the page may not read reads as unset.
  }
  return preference(stored) ?? "baseline";
};

export interface PlayerChoice {
  format: StreamFormat;
  preference: PlayerPreference;
  /** The window can run the low-latency player. */
  lowLatency: boolean;
}

// The player for a stream: the low-latency player for a unit-format
// stream when preferred and supported, else the baseline; none for a
// format this UI doesn't play.
export const choosePlayer = ({ format, preference, lowLatency }: PlayerChoice): PlayerKind => {
  switch (format) {
    case StreamFormat.Units:
      return preference === "lowlatency" && lowLatency ? PlayerKind.LowLatency : PlayerKind.Units;
    default:
      return PlayerKind.None;
  }
};
