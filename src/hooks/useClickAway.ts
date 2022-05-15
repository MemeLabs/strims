// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { RefObject, useEffect, useRef } from "react";

const defaultEvents = ["mousedown", "touchstart"];

const useClickAway = <E extends Event = Event>(
  ref: RefObject<HTMLElement | null> | RefObject<HTMLElement | null>[],
  onClickAway: (event: E) => void,
  events: string[] = defaultEvents
): void => {
  const savedCallback = useRef(onClickAway);
  useEffect(() => {
    savedCallback.current = onClickAway;
  }, [onClickAway]);

  useEffect(() => {
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
  }, [events, ref]);
};

export default useClickAway;
