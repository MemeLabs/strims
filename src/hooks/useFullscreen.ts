// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { useEffect, useState } from "react";

// Safari before 16.4 has the Fullscreen API with webkit prefixes only.
interface WebkitDocument {
  webkitFullscreenElement?: Element | null;
  webkitExitFullscreen?: () => Promise<void> | void;
}

interface WebkitElement {
  webkitRequestFullscreen?: () => Promise<void> | void;
}

const fullscreenElement = (): Element | null =>
  document.fullscreenElement ?? (document as WebkitDocument).webkitFullscreenElement ?? null;

// Shows `el` (the document's root without one) fullscreen, or leaves
// fullscreen if something is.
export const toggleFullscreen = async (el: Element = document.documentElement): Promise<void> => {
  if (fullscreenElement()) {
    const doc = document as Document & WebkitDocument;
    await (doc.exitFullscreen ? doc.exitFullscreen() : doc.webkitExitFullscreen?.());
  } else {
    const target = el as Element & WebkitElement;
    await (target.requestFullscreen
      ? target.requestFullscreen()
      : target.webkitRequestFullscreen?.());
  }
};

const CHANGE_EVENTS = ["fullscreenchange", "webkitfullscreenchange"];

// Whether the document shows an element fullscreen, and toggleFullscreen.
const useFullscreen = (): [boolean, typeof toggleFullscreen] => {
  const [isFullscreen, setFullscreen] = useState(() => fullscreenElement() !== null);

  useEffect(() => {
    const handleChange = () => setFullscreen(fullscreenElement() !== null);
    CHANGE_EVENTS.forEach((e) => document.addEventListener(e, handleChange));
    return () => CHANGE_EVENTS.forEach((e) => document.removeEventListener(e, handleChange));
  }, []);

  return [isFullscreen, toggleFullscreen];
};

export default useFullscreen;
