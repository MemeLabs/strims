// based on the react useEvent rfc
// see: https://github.com/reactjs/rfcs/blob/useevent/text/0000-useevent.md
// discussion: https://github.com/facebook/react/issues/14099

import { useCallback, useLayoutEffect, useMemo, useRef } from "react";

export const useStableCallback = <T extends (...args: any[]) => void>(callback: T) => {
  const ref = useRef<T>(null);

  useLayoutEffect(() => {
    ref.current = callback;
  }, [callback]);

  return useCallback((...args: Parameters<T>) => ref.current(...args), []);
};

export const useStableCallbacks = <T extends { [key: string]: (...args: any[]) => void }>(
  callbacks: T
) => {
  const ref = useRef<T>(null);

  useLayoutEffect(() => {
    ref.current = callbacks;
  }, [callbacks]);

  return useMemo(() => {
    const proxy = {} as { [K in keyof T]: (...args: Parameters<T[K]>) => void };
    for (const key in callbacks) {
      proxy[key] = (...args) => ref.current[key](...args);
    }
    return proxy;
  }, [Object.keys(callbacks).join("\n")]);
};
