export const setInStateMap = <K extends string, V, T extends Record<K, Map<string, V>>>(
  setState: React.Dispatch<React.SetStateAction<T>>,
  c: K,
  k: string,
  action: (value: V, state: T) => V
) =>
  setState((state) => ({
    ...state,
    [c]: new Map(state[c]).set(k, action(state[c].get(k), state)),
  }));

export const updateInStateMap = <K extends string, V, T extends Record<K, Map<string, V>>>(
  setState: React.Dispatch<React.SetStateAction<T>>,
  c: K,
  k: string,
  action: (value: V, state: T) => V
) =>
  setState((state) => {
    const prev = state[c].get(k);
    return prev ? { ...state, [c]: new Map(state[c]).set(k, action(prev, state)) } : state;
  });
