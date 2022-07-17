// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

export const applyActionInStateMap = <
  K extends string,
  V,
  T extends Record<K, Map<string, V>>,
  A extends any[]
>(
  state: T,
  c: K,
  k: string,
  action: (value: V, state: T, ...args: A) => V | void,
  ...args: A
): T => {
  const prev = state[c].get(k);
  if (!prev) {
    return state;
  }

  const map = new Map(state[c]);
  const next = action(prev, state, ...args);
  if (next) {
    map.set(k, next);
  } else {
    map.delete(k);
  }
  return { ...state, [c]: map };
};

export const deleteFromStateMap = <K extends string, V, T extends Record<K, Map<string, V>>>(
  state: T,
  c: K,
  k: string
): T => {
  const map = new Map(state[c]);
  map.delete(k);
  return { ...state, [c]: map };
};

export const setInStateMap = <K extends string, V, T extends Record<K, Map<string, V>>>(
  state: T,
  c: K,
  k: string,
  v: V
): T => ({ ...state, [c]: new Map(state[c]).set(k, v) });
