// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The UI's choices that belong to the device rather than the profile, kept
// in the page's local storage: each device keeps its own.

export interface DevicePreferences {
  /**
   * The chat on the player's other side: on a desktop the chat on the left,
   * on a phone the drawer opening from the left edge, for a left thumb.
   */
  swapMainPanels: boolean;
}

// load gives the stored preferences, the defaults for any not stored or
// unreadable; save stores `preferences` over the stored ones, or nothing
// where storage fails.
export interface DevicePreferenceStore {
  load(): DevicePreferences;
  save(preferences: Partial<DevicePreferences>): void;
}

const STORAGE_KEY = "devicePreferences";

// A store over `storage`, which returns null where the page has none. It
// keeps the stored keys it doesn't know, a later build's.
export const createDevicePreferenceStore = (
  storage: () => Storage | null
): DevicePreferenceStore => {
  // The stored object, empty where nothing readable is stored.
  const read = (): Record<string, unknown> => {
    try {
      const stored: unknown = JSON.parse(storage()?.getItem(STORAGE_KEY) ?? "{}");
      return stored && typeof stored === "object" ? (stored as Record<string, unknown>) : {};
    } catch {
      return {};
    }
  };

  return {
    load: () => ({
      swapMainPanels: read().swapMainPanels === true,
    }),
    save: (preferences) => {
      try {
        storage()?.setItem(STORAGE_KEY, JSON.stringify({ ...read(), ...preferences }));
      } catch {
        // The preferences last this page only.
      }
    },
  };
};

// The page's local storage, or none where the page may not use it.
const localStorageOrNull = (): Storage | null => {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

// The device's preferences in the page's local storage.
export const devicePreferences = createDevicePreferenceStore(localStorageOrNull);
