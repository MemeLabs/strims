// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

import { LinkedProfile } from "../apis/strims/auth/v1/auth";

/** The local storage key of the linked profile the watch page signs in with. */
export const PROFILE_ID_KEY = "watch.profileId";

const NAME_ALPHABET = "abcdefghijklmnopqrstuvwxyz0123456789";
const NAME_SUFFIX_LENGTH = 10;

const storage = (): Storage | null => {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

/** The id of the linked profile the page signed in with last. */
export const rememberedProfileId = (): string | null => {
  try {
    return storage()?.getItem(PROFILE_ID_KEY) ?? null;
  } catch {
    return null;
  }
};

export const rememberProfileId = (id: bigint): void => {
  try {
    storage()?.setItem(PROFILE_ID_KEY, id.toString());
  } catch {
    // The next visit picks the newest profile instead.
  }
};

/** A name for a new anonymous profile, valid under `validNamePattern`. */
export const viewerName = (
  random = crypto.getRandomValues(new Uint8Array(NAME_SUFFIX_LENGTH))
): string =>
  "viewer_" + Array.from(random, (b) => NAME_ALPHABET[b % NAME_ALPHABET.length]).join("");

/**
 * A new anonymous profile's password. Nobody signs in with it: the page
 * signs in with the profile key the node gives back for a persisted login.
 */
export const throwawayPassword = (): string =>
  Base64.fromUint8Array(crypto.getRandomValues(new Uint8Array(32)));

/**
 * The local profiles the page may sign in with, in the order it tries
 * them: the one it used last, then the rest newest first. Only profiles on
 * the node in the page whose key is stored qualify.
 */
export const signInCandidates = (
  profiles: LinkedProfile[],
  rememberedId: string | null
): LinkedProfile[] =>
  profiles
    .filter((p) => !p.serverAddress && p.credentials.case === LinkedProfile.CredentialsCase.KEY)
    .sort((a, b) => {
      const ra = a.id.toString() === rememberedId;
      const rb = b.id.toString() === rememberedId;
      if (ra !== rb) return ra ? -1 : 1;
      return a.id < b.id ? 1 : a.id > b.id ? -1 : 0;
    });
