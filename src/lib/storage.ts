// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The page's local storage, for the UI's components, hooks and contexts,
// none of which touch it themselves. A page may have none (a sandboxed
// frame, blocked site data), or one that throws when read or written (a
// full quota): either reads as nothing stored, and a write to it is lost.

// The page's local storage, or null where the page may not use it.
export const localStorageOrNull = (): Storage | null => {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

// The item stored under `key` in `storage`, or null where none is or
// reading fails.
export const readItem = (
  key: string,
  storage: () => Storage | null = localStorageOrNull
): string | null => {
  try {
    return storage()?.getItem(key) ?? null;
  } catch {
    return null;
  }
};

// Stores `value` under `key` in `storage`, or nothing where writing fails.
export const writeItem = (
  key: string,
  value: string,
  storage: () => Storage | null = localStorageOrNull
): void => {
  try {
    storage()?.setItem(key, value);
  } catch {
    // The value lasts this page only.
  }
};
