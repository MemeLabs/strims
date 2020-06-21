import { useRef, useState } from "react";

import useReady from "./useReady";

export const VideoReadyState = {
  // No information is available about the media resource.
  HAVE_NOTHING: 0,
  // Enough of the media resource has been retrieved that the metadata attributes
  // are initialized. Seeking will no longer raise an exception.
  HAVE_METADATA: 1,
  // Data is available for the current playback position, but not enough to
  // actually play more than one frame.
  HAVE_CURRENT_DATA: 2,
  // Data for the current playback position as well as for at least a little
  // bit of time into the future is available (in other words, at least two frames of video, for example).
  HAVE_FUTURE_DATA: 3,
  // Enough data is available—and the download rate is high enough—that the
  // media can be played through to the end without interruption.
  HAVE_ENOUGH_DATA: 4,
};

const useVideo = () => {
  const ref = useRef();
  const [loaded, setLoaded] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [paused, setPaused] = useState(false);
  const [ended, setEnded] = useState(true);
  const [waiting, setWaiting] = useState(true);
  const [muted, setMuted] = useState(null);
  const [volume, unsafelySetVolume] = useState(null);
  const [savedVolume, setSavedVolume] = useState(null);
  const [readyState, setReadyState] = useState(0);
  const [bufferStart, setBufferStart] = useState(0);
  const [bufferEnd, setBufferEnd] = useState(1);
  const [duration, setDuration] = useState(0);
  const [currentTime, unsafelySetCurrentTime] = useState(0);
  const [seekableStart, setSeekableStart] = useState(0);
  const [seekableEnd, setSeekableEnd] = useState(0);

  useReady(() => {
    setMuted(ref.current.muted);
    setVolume(ref.current.volume);
    setPaused(ref.current.paused);
    setReadyState(ref.current.readyState);
  }, [ref.current]);

  const onEnded = () => {
    setPlaying(false);
    setEnded(false);
    setWaiting(false);
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

  const onLoadedMetadata = (e) => {
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
      ref.current.muted = true;
      try {
        await ref.current.play();
      } catch (e) {
        console.warn("error playing video", e);
      }
    }
  };

  const mute = () => {
    setSavedVolume(ref.current.volume);
    ref.current.volume = 0;
  };

  const unmute = () => {
    ref.current.volume = savedVolume || 0.5;
  };

  const setVolume = (volume) => {
    if (ref.current) {
      const clampedVolume = Math.max(0, Math.min(1, volume));
      unsafelySetVolume(clampedVolume);
      ref.current.volume = clampedVolume;
    }
  };

  const setCurrentTime = (time) => {
    if (!ref.current) {
      return;
    }

    ref.current.currentTime = time;
    unsafelySetCurrentTime(time);
  };

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

  const src = ref.current && ref.current.src;
  const setSrc = (src) => ref.current && src && (ref.current.src = src);

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
      supportPiP,
      pip,
      src,
    },
    {
      ref,
      onEnded,
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
      pause: () => ref.current && ref.current.pause(),
      play,
      setCurrentTime,
      setVolume,
      togglePiP,
      setSrc,
    },
  ];
};

export default useVideo;
