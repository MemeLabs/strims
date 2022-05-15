// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useRef } from "react";

const useUpdate = (effect: () => void, deps: [any]): void => {
  const init = useRef(false);

  useEffect(() => {
    if (init.current) {
      effect();
    }
  }, deps);

  useEffect(() => {
    init.current = true;
  }, []);
};

export default useUpdate;
