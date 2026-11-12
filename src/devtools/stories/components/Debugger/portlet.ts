// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect } from "react";

// The Portlet portals into document.body at the page's top left, 400 px
// square: outside the story's frame (and its screenshot). Stories move it
// over the frame, and size it, the way a viewer would: by its resize handle
// and its header.

const DEFAULT_SIZE = 400;
const MARGIN = 16;

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
  /** Its offset into the frame (default 16 px each way). */
  x?: number;
  y?: number;
}

// Once `find` gives an element the portlet's children rendered, moves the
// portlet into the story's frame, (`x`, `y`) from its top left, `width` x
// `height`.
export const usePortletInFrame = (
  find: () => Element | null,
  { width = DEFAULT_SIZE, height = DEFAULT_SIZE, x = MARGIN, y = MARGIN }: PortletPlacement = {}
): void =>
  useEffect(() => {
    let frame = 0;
    const place = () => {
      const content = find();
      const parts = content && portletParts(content);
      const root = document.querySelector("[data-story-root]");
      if (!parts || !root) {
        frame = requestAnimationFrame(place);
        return;
      }
      const { header, resizeHandle, container } = parts;
      const dw = width - DEFAULT_SIZE;
      const dh = height - DEFAULT_SIZE;
      fire(resizeHandle, "mousedown", 0, 0);
      fire(container, "mousemove", dw, dh);
      fire(container, "mouseup", dw, dh);

      const { left, top } = root.getBoundingClientRect();
      const to = [left + x + window.scrollX, top + y + window.scrollY] as const;
      fire(header, "mousedown", 0, 0);
      fire(container, "mousemove", ...to);
      fire(container, "mouseup", ...to);
    };
    frame = requestAnimationFrame(place);
    return () => cancelAnimationFrame(frame);
  }, []);
