// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { RefObject, useEffect, useRef } from "react";

const defaultEvents = ["mousedown", "touchstart"];

interface Options {
  events?: string[];
  enable?: boolean;
}

const useClickAway = <E extends Event = Event>(
  ref: RefObject<HTMLElement | null> | RefObject<HTMLElement | null>[],
  onClickAway: (event: E) => void,
  options?: Options
): void => {
  const { events, enable } = { events: defaultEvents, enable: true, ...options };

  const savedCallback = useRef(onClickAway);
  useEffect(() => {
    savedCallback.current = onClickAway;
  }, [onClickAway]);

  useEffect(() => {
    if (!enable) {
      return;
    }

    const refs = Array.isArray(ref) ? ref : [ref];

    const handler = (event: E) => {
      const target = event.target as Element;
      if (!document.contains(target)) {
        return;
      }
      for (const ref of refs) {
        if (ref.current?.contains(target)) {
          return;
        }
      }

      savedCallback.current(event);
    };

    for (const eventName of events) {
      document.addEventListener(eventName, handler);
    }

    return () => {
      for (const eventName of events) {
        document.removeEventListener(eventName, handler);
      }
    };
  }, [events, enable, ref]);
};

export default useClickAway;

const suppressClickAwayHandlers = Object.seal({
  onMouseDown: (e: React.MouseEvent) => e.stopPropagation(),
  onClick: (e: React.MouseEvent) => e.stopPropagation(),
  onTouchStart: (e: React.TouchEvent) => e.stopPropagation(),
});

// suppress propagation of pointer events that would trigger useClickAway. for
// example in a context menu anchored in a modal that unmounts when it loses
// focus because the menu is rendered in a portal the target detection doesn't
// work
export const suppressClickAway = () => suppressClickAwayHandlers;
