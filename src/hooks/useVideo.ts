// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { ComponentProps, useEffect, useState } from "react";

import { CUES_CHANGED, CueKind, cueKinds, cueTrack } from "../lib/media/textTracks";
import useReady from "./useReady";

export enum VideoReadyState {
  // No information is available about the media resource.
  HAVE_NOTHING = 0,
  // Enough of the media resource has been retrieved that the metadata attributes
  // are initialized. Seeking will no longer raise an exception.
  HAVE_METADATA = 1,
  // Data is available for the current playback position, but not enough to
  // actually play more than one frame.
  HAVE_CURRENT_DATA = 2,
  // Data for the current playback position as well as for at least a little
  // bit of time into the future is available (in other words, at least two frames of video, for example).
  HAVE_FUTURE_DATA = 3,
  // Enough data is available—and the download rate is high enough—that the
  // media can be played through to the end without interruption.
  HAVE_ENOUGH_DATA = 4,
}

const VOLUME_STORAGE_KEY = "volume";
const CAPTIONS_STORAGE_KEY = "captions";

/** The cues the viewer shows: none, or those of a kind, the other's when the stream has only it. */
export type CaptionChoice = "off" | CueKind;

const loadCaptions = (): CaptionChoice => {
  try {
    const saved = localStorage.getItem(CAPTIONS_STORAGE_KEY);
    return saved === "subtitles" || saved === "captions" ? saved : "off";
  } catch {
    return "off";
  }
};

const saveCaptions = (choice: CaptionChoice) => {
  try {
    localStorage.setItem(CAPTIONS_STORAGE_KEY, choice);
  } catch {
    // The choice lasts this page only.
  }
};

/** The kind of cues showing for `choice`, of the stream's `kinds`. */
export const shownCues = (choice: CaptionChoice, kinds: CueKind[]): CueKind | null => {
  if (choice === "off" || !kinds.length) return null;
  return kinds.includes(choice) ? choice : kinds[0];
};

export interface VideoState {
  readyState: number;
  loaded: boolean;
  playing: boolean;
  paused: boolean;
  ended: boolean;
  waiting: boolean;
  muted: boolean;
  volume: number;
  bufferStart: number;
  bufferEnd: number;
  duration: number;
  currentTime: number;
  seekableStart: number;
  seekableEnd: number;
  videoHeight: number;
  videoWidth: number;
  supportPiP: boolean;
  pip: boolean;
  src: string;
  error: MediaError | null;
  /** The kinds of cues the stream has shown, and those showing: null when off. */
  cueKinds: CueKind[];
  captions: CueKind | null;
}

export interface VideoControls {
  mute: () => void;
  unmute: () => void;
  pause: () => void;
  play: () => void;
  setCurrentTime: (value: number) => void;
  setVolume: (value: number) => void;
  togglePiP: () => Promise<void>;
  setSrc: (src: string) => void;
  /** Shows the next kind of cues the stream has, or none after the last; the choice is kept on the device. */
  cycleCaptions: () => void;
}

const useVideo = (
  ref: React.MutableRefObject<HTMLVideoElement>
): [VideoState, ComponentProps<"video">, VideoControls] => {
  const [src, setSrc] = useState<string>(null);
  const [loaded, setLoaded] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [paused, setPaused] = useState(false);
  const [ended, setEnded] = useState(true);
  const [waiting, setWaiting] = useState(true);
  const [muted, setMuted] = useState(true);
  const [volume, unsafelySetVolume] = useState(0);
  const [savedVolume, setSavedVolume] = useState(0);
  const [readyState, setReadyState] = useState<VideoReadyState>(VideoReadyState.HAVE_NOTHING);
  const [bufferStart, setBufferStart] = useState(0);
  const [bufferEnd, setBufferEnd] = useState(1);
  const [duration, setDuration] = useState(0);
  const [currentTime, unsafelySetCurrentTime] = useState(0);
  const [seekableStart, setSeekableStart] = useState(0);
  const [seekableEnd, setSeekableEnd] = useState(0);
  const [error, setError] = useState<MediaError>(null);
  const [kinds, setKinds] = useState<CueKind[]>([]);
  const [captionChoice, setCaptionChoice] = useState<CaptionChoice>(loadCaptions);

  useReady(() => {
    const savedVolume = localStorage.getItem(VOLUME_STORAGE_KEY);
    if (savedVolume !== "") {
      const volume = parseFloat(savedVolume) || 0;
      ref.current.muted = volume === 0;
      ref.current.volume = volume;
    }

    setMuted(ref.current.muted);
    setVolume(ref.current.volume);
    setReadyState(ref.current.readyState);
  }, [ref.current]);

  useReady(() => {
    setLoaded(false);
    setPlaying(false);
    setEnded(true);
    setWaiting(true);
  }, [src]);

  // The kinds of cues the stream's players showed, until the video's
  // source goes.
  useEffect(() => {
    const video = ref.current;
    if (!video) return;
    const changed = () =>
      setKinds((was) => {
        const now = cueKinds(video);
        const all = (["subtitles", "captions"] as const).filter(
          (k) => was.includes(k) || now.includes(k)
        );
        return all.length === was.length ? was : all;
      });
    const emptied = () => setKinds([]);
    video.addEventListener(CUES_CHANGED, changed);
    video.addEventListener("emptied", emptied);
    return () => {
      video.removeEventListener(CUES_CHANGED, changed);
      video.removeEventListener("emptied", emptied);
    };
  }, [ref.current]);

  const captions = shownCues(captionChoice, kinds);
  useEffect(() => {
    const video = ref.current;
    if (!video) return;
    for (const k of ["subtitles", "captions"] as const) {
      const track = cueTrack(video, k);
      if (track) track.mode = k === captions ? "showing" : "hidden";
    }
  }, [ref.current, captions, kinds]);

  const cycleCaptions = () => {
    const order: CaptionChoice[] = ["off", ...kinds];
    const next = order[(order.indexOf(captions ?? "off") + 1) % order.length];
    setCaptionChoice(next);
    saveCaptions(next);
  };

  const onEnded = () => {
    setPlaying(false);
    setEnded(false);
    setWaiting(false);
  };

  const onError = () => {
    setPlaying(false);
    setEnded(true);
    setWaiting(false);
    setError(ref.current.error);
  };

  const onPause = () => {
    setPlaying(false);
    setPaused(true);
  };

  const onPlaying = () => {
    setPaused(false);
    setPlaying(true);
    setReadyState(ref.current.readyState);
  };

  const onCanPlay = () => {
    setWaiting(false);
    setLoaded(true);
    setReadyState(ref.current.readyState);
  };

  const onCanPlayThrough = () => {
    setWaiting(false);
    setLoaded(true);
    setReadyState(ref.current.readyState);
  };

  const onVolumeChange = () => {
    setVolume(ref.current.volume);
  };

  const onWaiting = () => {
    setPlaying(false);
    setWaiting(true);
    setReadyState(ref.current.readyState);
  };

  const onDurationChange = () => {
    setReadyState(ref.current.readyState);
  };

  const onLoadedMetadata = () => {
    setReadyState(ref.current.readyState);
  };

  const onLoadedData = () => {
    setReadyState(ref.current.readyState);
  };

  const onTimeUpdate = () => {
    const video = ref.current;
    const { buffered, seekable } = video;

    if (buffered.length === 0 || seekable.length === 0) {
      return;
    }

    const bufferEnd = buffered.end(buffered.length - 1);

    setBufferStart(buffered.start(0));
    setBufferEnd(bufferEnd);
    setDuration(video.duration);
    unsafelySetCurrentTime(video.currentTime);
    setSeekableStart(seekable.start(0));
    setSeekableEnd(seekable.end(0));
  };

  const play = async () => {
    try {
      await ref.current.play();
    } catch (e) {
      try {
        mute();
        await ref.current.play();
      } catch (e) {
        console.warn("error playing video", e);
      }
    }
  };

  const mute = () => {
    setSavedVolume(ref.current.volume);
    ref.current.muted = true;
    ref.current.volume = 0;
  };

  const unmute = () => {
    ref.current.muted = false;
    ref.current.volume = savedVolume || 0.5;
  };

  const pause = () => {
    ref.current?.pause();
  };

  const setVolume = (volume: number) => {
    const clampedVolume = Math.max(0, Math.min(1, volume));
    localStorage.setItem(VOLUME_STORAGE_KEY, clampedVolume.toString());

    if (ref.current) {
      unsafelySetVolume(clampedVolume);
      ref.current.volume = clampedVolume;
      ref.current.muted = clampedVolume === 0;
    }
  };

  const setCurrentTime = (time: number) => {
    if (!ref.current) {
      return;
    }

    ref.current.currentTime = time;
    unsafelySetCurrentTime(time);
  };

  const videoHeight = ref.current?.videoHeight;
  const videoWidth = ref.current?.videoWidth;

  const supportPiP =
    document.pictureInPictureEnabled && ref.current && !ref.current.disablePictureInPicture;
  const pip = ref.current === document.pictureInPictureElement;

  const togglePiP = async () => {
    try {
      if (pip) {
        await document.exitPictureInPicture();
      } else {
        await ref.current.requestPictureInPicture();
      }
    } catch (e) {
      console.warn("error opening pip", e);
    }
  };

  return [
    {
      readyState,
      loaded,
      playing,
      paused,
      ended,
      waiting,
      muted,
      volume,
      bufferStart,
      bufferEnd,
      duration,
      currentTime,
      seekableStart,
      seekableEnd,
      videoHeight,
      videoWidth,
      supportPiP,
      pip,
      src,
      error,
      cueKinds: kinds,
      captions,
    },
    {
      ref,
      src,
      onEnded,
      onError,
      onPause,
      onPlaying,
      onCanPlay,
      onCanPlayThrough,
      onVolumeChange,
      onWaiting,
      onDurationChange,
      onLoadedMetadata,
      onLoadedData,
      onTimeUpdate,
    },
    {
      mute,
      unmute,
      pause,
      play,
      setCurrentTime,
      setVolume,
      togglePiP,
      setSrc,
      cycleCaptions,
    },
  ];
};

export default useVideo;
