// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { MutableRefObject, RefCallback, useCallback } from "react";

type Ref<T> = RefCallback<T> | MutableRefObject<T>;

const useRefs = <T>(...refs: Ref<T>[]) =>
  useCallback((e: T) => {
    for (const ref of refs) {
      if (ref instanceof Function) {
        ref(e);
      } else if (ref) {
        ref.current = e;
      }
    }
  }, refs);

export default useRefs;
