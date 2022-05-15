// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect } from "react";

const useReady = (effect: () => void | (() => void), deps: any[]): void =>
  useEffect(() => {
    for (let i = 0; i < deps.length; i++) {
      if (deps[i] == null) {
        return;
      }
    }

    return effect();
  }, deps);

export default useReady;
