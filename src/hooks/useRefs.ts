// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useCallback } from "react";

import { Ref, setRef } from "../lib/ref";

const useRefs = <T>(...refs: Ref<T>[]) =>
  useCallback((e: T) => {
    for (const ref of refs) {
      setRef(ref, e);
    }
  }, refs);

export default useRefs;
