// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The page's side of its frames (protocol.ts): the story's iframe and the
// hidden one that builds the index.

import clsx from "clsx";
import React, { useEffect, useRef, useState } from "react";

import type { IndexData } from "./catalog";
import { DEVICES, Device } from "./devices";
import { FrameMessage, INDEX_ROUTE, isFrameMessage } from "./protocol";
import type { SessionSnapshot } from "./recorder";

// A same-origin document's hash route as a URL, this page's path.
export const documentUrl = (route: string): string => `${location.pathname}#${route}`;

// Messages `frame`'s document posts, whichever document it holds.
const useFrameMessages = (
  frame: React.RefObject<HTMLIFrameElement>,
  onMessage: (m: FrameMessage) => void
) => {
  const handler = useRef(onMessage);
  handler.current = onMessage;
  useEffect(() => {
    const listener = (e: MessageEvent) => {
      if (
        e.origin === location.origin &&
        frame.current &&
        e.source === frame.current.contentWindow &&
        isFrameMessage(e.data)
      ) {
        handler.current(e.data);
      }
    };
    window.addEventListener("message", listener);
    return () => window.removeEventListener("message", listener);
  }, []);
};

export interface StoryIframeProps {
  /** The frame document's hash route (protocol.frameRoute). */
  src: string;
  device: Device;
  onSnapshot: (snapshot: SessionSnapshot) => void;
  onRouterPath: (path: string | null) => void;
}

// The story's frame document, sized to its device (pc fills the stage).
// A change of `src` loads a new document; the caller keys it to remount.
export const StoryIframe: React.FC<StoryIframeProps> = ({
  src,
  device,
  onSnapshot,
  onRouterPath,
}) => {
  const ref = useRef<HTMLIFrameElement>(null);
  const [state, setState] = useState<string>("loading");
  useFrameMessages(ref, (m) => {
    if (m.storybook === "story") {
      setState(m.snapshot.state);
      onSnapshot(m.snapshot);
    } else if (m.storybook === "router") {
      onRouterPath(m.path);
    }
  });
  const shape = DEVICES[device];
  return (
    <iframe
      ref={ref}
      src={documentUrl(src)}
      title="story"
      className={clsx("storybook__iframe", `storybook__iframe--${device}`)}
      style={shape ? { width: `${shape.width}px`, height: `${shape.height}px` } : undefined}
      data-story-state={state}
    />
  );
};

export interface StoryIndex {
  index: IndexData | null;
  error: string | null;
  /** The hidden iframe that builds it, to render while `index` is null. */
  frame: React.ReactNode;
}

// The story index, built in a hidden frame document so this page never
// evaluates a story module (nor loads its styles).
export const useStoryIndex = (): StoryIndex => {
  const ref = useRef<HTMLIFrameElement>(null);
  const [index, setIndex] = useState<IndexData>(null);
  const [error, setError] = useState<string>(null);
  useFrameMessages(ref, (m) => {
    if (m.storybook === "index") {
      setIndex(m.index);
    } else if (m.storybook === "index-error") {
      setError(m.error);
    }
  });
  const frame =
    index || error ? null : (
      <iframe ref={ref} src={documentUrl(INDEX_ROUTE)} title="story index" hidden />
    );
  return { index, error, frame };
};
