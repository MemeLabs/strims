// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoPlayer.stories.scss";

import clsx from "clsx";
import React, { ReactNode, useMemo, useState } from "react";

import type { VideoControls, VideoState } from "../../../../hooks/useVideo";
import { fixtures } from "../../../storybook";

// What useVideo says of a video playing a live stream: 1080p, a minute
// buffered, 70% volume.
export const videoState = (over: Partial<VideoState> = {}): VideoState => ({
  readyState: 4,
  loaded: true,
  playing: true,
  paused: false,
  ended: false,
  waiting: false,
  muted: false,
  volume: 0.7,
  bufferStart: 0,
  bufferEnd: 60,
  duration: Infinity,
  currentTime: 42,
  seekableStart: 0,
  seekableEnd: 60,
  videoHeight: 1080,
  videoWidth: 1920,
  supportPiP: true,
  pip: false,
  src: "",
  error: null,
  cueKinds: [],
  captions: null,
  ...over,
});

// A video's state and controls that act on it, without a video: play,
// pause, mute, volume, seeking and captions change the state.
export const useFakeVideo = (initial: Partial<VideoState> = {}): [VideoState, VideoControls] => {
  const [state, setState] = useState(() => videoState(initial));
  const controls = useMemo<VideoControls>(() => {
    const set = (over: Partial<VideoState>) => setState((s) => ({ ...s, ...over }));
    return {
      mute: () => set({ muted: true, volume: 0 }),
      unmute: () => set({ muted: false, volume: 0.7 }),
      pause: () => set({ playing: false, paused: true }),
      play: () => set({ playing: true, paused: false }),
      setCurrentTime: (currentTime) => set({ currentTime }),
      setVolume: (v) => {
        const volume = Math.max(0, Math.min(1, v));
        set({ volume, muted: volume === 0 });
      },
      togglePiP: () => {
        setState((s) => ({ ...s, pip: !s.pip }));
        return Promise.resolve();
      },
      setSrc: (src) => set({ src }),
      cycleCaptions: () =>
        setState((s) => {
          const kinds = [null, ...s.cueKinds];
          return { ...s, captions: kinds[(kinds.indexOf(s.captions) + 1) % kinds.length] };
        }),
    };
  }, []);
  return [state, controls];
};

interface PlayerFrameProps {
  /** A thumbnail behind the controls, standing for the picture. */
  picture?: boolean;
  /** Shows the sliders as they show under the pointer. */
  hover?: boolean;
  children: ReactNode;
}

// A 16:9 box for a player's controls, as the player positions them.
export const PlayerFrame: React.FC<PlayerFrameProps> = ({ picture = true, hover, children }) => (
  <div className="video_story">
    <div
      className={clsx("video_player", "video_story__player", {
        "video_story__player--hover": hover,
      })}
      style={picture ? { backgroundImage: `url(${fixtures.thumbnailUrls[4]})` } : undefined}
    >
      {children}
    </div>
  </div>
);
