// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect } from "react";

// The Portlet portals into document.body at the page's top left, 400 px
// square. Stories size it, and may move it, the way a viewer would: by its
// resize handle and its header.

const DEFAULT_SIZE = 400;

const fire = (el: Element, type: string, x: number, y: number) =>
  el.dispatchEvent(
    new MouseEvent(type, { bubbles: true, cancelable: true, view: window, clientX: x, clientY: y })
  );

// The portlet around `content`, an element its children rendered.
const portletParts = (content: Element) => {
  // children > size wrapper > scroll area > the window's column
  const column = content.parentElement?.parentElement?.parentElement;
  const positioned = column?.parentElement?.parentElement;
  const container = positioned?.parentElement;
  if (!column || !positioned || !container) {
    return null;
  }
  return {
    header: column.firstElementChild,
    resizeHandle: positioned.lastElementChild?.lastElementChild,
    container,
  };
};

export interface PortletPlacement {
  width?: number;
  height?: number;
  /** Where its top left goes (default where it opens, the page's corner). */
  x?: number;
  y?: number;
}

// Once `find` gives an element the portlet's children rendered, sizes the
// portlet `width` x `height` and moves it to (`x`, `y`).
export const usePortletPlacement = (
  find: () => Element | null,
  { width = DEFAULT_SIZE, height = DEFAULT_SIZE, x = 0, y = 0 }: PortletPlacement = {}
): void =>
  useEffect(() => {
    let frame = 0;
    const place = () => {
      const content = find();
      const parts = content && portletParts(content);
      if (!parts) {
        frame = requestAnimationFrame(place);
        return;
      }
      const { header, resizeHandle, container } = parts;
      if (width !== DEFAULT_SIZE || height !== DEFAULT_SIZE) {
        const dw = width - DEFAULT_SIZE;
        const dh = height - DEFAULT_SIZE;
        fire(resizeHandle, "mousedown", 0, 0);
        fire(container, "mousemove", dw, dh);
        fire(container, "mouseup", dw, dh);
      }
      if (x || y) {
        fire(header, "mousedown", 0, 0);
        fire(container, "mousemove", x, y);
        fire(container, "mouseup", x, y);
      }
    };
    frame = requestAnimationFrame(place);
    return () => cancelAnimationFrame(frame);
  }, []);
