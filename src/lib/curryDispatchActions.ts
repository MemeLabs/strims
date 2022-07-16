// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

type RestParameters<T extends (...args: any) => any> = T extends (_: any, ...args: infer P) => any
  ? P
  : never;

const curryDispatchActions = <S, T extends { [key: string]: (state: S, ...args: any[]) => S }>(
  setState: React.Dispatch<React.SetStateAction<S>>,
  callbacks: T
) => {
  const proxy = {} as { [K in keyof T]: (...args: RestParameters<T[K]>) => void };
  for (const key in callbacks) {
    proxy[key] = (...args) => setState((s) => callbacks[key](s, ...args));
  }
  return proxy;
};

export default curryDispatchActions;
