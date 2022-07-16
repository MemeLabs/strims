// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

export const applyActionInStateMap = <K extends string, V, T extends Record<K, Map<string, V>>>(
  state: T,
  c: K,
  k: string,
  action: (value: V, state: T) => V
) => {
  const prev = state[c].get(k);
  return prev ? { ...state, [c]: new Map(state[c]).set(k, action(prev, state)) } : state;
};
