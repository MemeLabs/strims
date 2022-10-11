// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// based on the react useEvent rfc
// see: https://github.com/reactjs/rfcs/blob/useevent/text/0000-useevent.md
// discussion: https://github.com/facebook/react/issues/14099

import { useCallback, useMemo, useRef } from "react";

export const useStableCallback = <P extends any[], R>(
  callback: (...args: P) => R
): ((...args: P) => R) => {
  const ref = useRef<(...args: P) => R>(null);
  ref.current = callback;

  return useCallback((...args: P) => ref.current(...args), []);
};

type ReturnTypes<T> = {
  [K in keyof T]: T[K] extends (...args: any[]) => infer R ? R : never;
}[keyof T];

export const useStableCallbacks = <
  R extends ReturnTypes<T>,
  T extends { [key: string]: (...args: any[]) => R }
>(
  callbacks: T
) => {
  const ref = useRef<T>(null);
  ref.current = callbacks;

  return useMemo(() => {
    const proxy = {} as { [K in keyof T]: (...args: Parameters<T[K]>) => ReturnType<T[K]> };
    for (const key in callbacks) {
      proxy[key] = (...args) => ref.current[key](...args);
    }
    return proxy;
  }, [Object.keys(callbacks).join("\n")]);
};
