// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import useResizeObserver from "@react-hook/resize-observer";
import * as React from "react";

const useSize = <T extends HTMLElement>(
  target: React.RefObject<T> | HTMLElement | (() => HTMLElement)
): DOMRectReadOnly => {
  const [size, setSize] = React.useState<DOMRectReadOnly>();

  React.useLayoutEffect(() => {
    if (target instanceof HTMLElement) {
      setSize(target.getBoundingClientRect());
    } else if (target instanceof Function) {
      setSize(target()?.getBoundingClientRect());
    } else {
      setSize(target?.current?.getBoundingClientRect());
    }
  }, [target]);

  if (target instanceof HTMLElement) {
    useResizeObserver(target, (entry) => setSize(entry.contentRect));
  } else if (target instanceof Function) {
    useResizeObserver(target(), (entry) => setSize(entry.contentRect));
  } else {
    useResizeObserver(target?.current, (entry) => setSize(entry.contentRect));
  }

  return size;
};

export default useSize;
