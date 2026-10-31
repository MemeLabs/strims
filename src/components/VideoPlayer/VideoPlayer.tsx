// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoPlayer.scss";

import clsx from "clsx";
import React, { useCallback, useEffect, useMemo, useRef } from "react";
import { MdLoop } from "react-icons/md";
import useFullscreen from "use-fullscreen";

import { useLayout } from "../../contexts/Layout";
import useIdleTimeout from "../../hooks/useIdleTimeout";
import useVideo from "../../hooks/useVideo";
import { lowLatencySupport } from "../../lib/media/lowlatency/support";
import { streamFormat } from "../../lib/media/mime";
import { PlayerKind, choosePlayer, preferredPlayer } from "../../lib/media/players";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import { useRemotePlayback } from "./RemotePlayback";
import { SwarmMediaProps, UnitsMedia } from "./SwarmMedia";
import { useRendition } from "./useRendition";
import VideoControls from "./VideoControls";

interface SwarmPlayerProps extends Pick<SwarmMediaProps, "networkKey" | "swarmUri" | "mimeType"> {
  defaultControlsVisible?: boolean;
  disableControls?: boolean;
  defaultAspectRatio?: string | number;
  handleClose: () => void;
  className?: string;
}

// The component that plays each kind of stream; none for a format this UI can't play.
const swarmMedia: Record<PlayerKind, React.FC<SwarmMediaProps> | null> = {
  [PlayerKind.LowLatency]: UnitsMedia,
  [PlayerKind.Units]: UnitsMedia,
  [PlayerKind.None]: null,
};

// The page's local storage, or none where the page may not use it.
const storage = (): Storage | null => {
  try {
    return window.localStorage;
  } catch {
    return null;
  }
};

const SwarmPlayer: React.FC<SwarmPlayerProps> = ({
  networkKey,
  swarmUri,
  mimeType,
  defaultControlsVisible,
  disableControls = false,
  defaultAspectRatio = "16/9",
  handleClose,
  className,
}) => {
  const rootRef = useRef();
  const [controlsHidden, renewControlsTimeout, clearControlsTimeout] = useIdleTimeout();
  const [isFullscreen, toggleFullscreen] = useFullscreen();
  const { theaterMode, toggleTheaterMode } = useLayout();
  const videoRef = useRef<HTMLVideoElement>();
  const [videoState, videoProps, videoControls] = useVideo(videoRef);
  const remote = useRemotePlayback({ networkKey, swarmUri, mimeType, videoRef });

  // Unit-format streams play through a player worker everywhere, the
  // low-latency player's when the viewer asks for it (?player=lowlatency,
  // or the "player" local storage setting) and the browser has what it
  // needs; other formats not at all.
  const format = streamFormat(mimeType);
  const kind = useMemo(
    () =>
      choosePlayer({
        format,
        preference: preferredPlayer(location.search, storage()),
        lowLatency: lowLatencySupport(),
      }),
    [format]
  );
  const SwarmMedia = swarmMedia[kind];
  // The rendition of the listing's stream that this browser plays best,
  // the listing's own failing others.
  const rendition = useRendition({
    swarmUri,
    mimeType,
    kind,
    videoRef,
    active: !!SwarmMedia && !remote.casting,
  });
  useEffect(() => {
    if (kind === PlayerKind.None) {
      // eslint-disable-next-line no-console
      console.warn("this stream's format can't play here:", mimeType);
    }
  }, [kind, mimeType]);

  useEffect(() => {
    console.log(">>>", videoState.error);
  }, [videoState.error]);

  const handleToggleFullscreen = useCallback(() => {
    if (typeof videoRef.current.webkitSetPresentationMode === "function") {
      videoRef.current.webkitSetPresentationMode("fullscreen");
    } else {
      void toggleFullscreen(rootRef.current);
    }
  }, []);

  useEffect(renewControlsTimeout, [videoState.volume]);

  let aspectRatio = defaultAspectRatio;
  if (videoState.videoWidth && videoState.videoHeight) {
    aspectRatio = `${videoState.videoWidth}/${videoState.videoHeight}`;
  }

  const controls =
    DEVICE_TYPE === DeviceType.Portable
      ? {
          showPlayButton: true,
          showVolume: true,
          showFullscreenButton: true,
          showCloseButton: true,
        }
      : {
          showPlayButton: true,
          showVolume: true,
          showPiPButton: true,
          showTheaterButton: true,
          showFullscreenButton: true,
        };

  return (
    <div
      className={clsx("video_player", className)}
      onMouseMove={renewControlsTimeout}
      onMouseLeave={clearControlsTimeout}
      ref={rootRef}
      style={{ aspectRatio }}
    >
      <video
        onClick={(e) => e.preventDefault()}
        onDoubleClick={handleToggleFullscreen}
        className="video_player__video"
        autoPlay
        playsInline
        {...videoProps}
      />
      {SwarmMedia && rendition.swarmUri && !remote.casting && (
        <SwarmMedia
          key={`${kind}:${networkKey}:${rendition.swarmUri}`}
          networkKey={networkKey}
          swarmUri={rendition.swarmUri}
          mimeType={rendition.mimeType}
          videoRef={videoRef}
          videoControls={videoControls}
          lowLatency={kind === PlayerKind.LowLatency}
          airPlaySource={remote.airPlaySource ?? undefined}
          onUnplayable={rendition.unplayable}
        />
      )}
      {remote.overlay}
      <div
        className={clsx({
          "video_player__waiting_spinner": true,
          "video_player__waiting_spinner--waiting": videoState.waiting,
          "video_player__waiting_spinner--loaded": videoState.loaded,
          "video_player__waiting_spinner--stalled": !videoState.playing && !videoState.paused,
        })}
      >
        <MdLoop />
      </div>
      <VideoControls
        videoState={videoState}
        videoControls={videoControls}
        visible={(!controlsHidden || defaultControlsVisible) && !disableControls}
        fullscreen={isFullscreen}
        toggleFullscreen={handleToggleFullscreen}
        theaterMode={theaterMode}
        toggleTheaterMode={toggleTheaterMode}
        close={handleClose}
        remoteButtons={remote.buttons}
        {...controls}
      />
    </div>
  );
};

export default SwarmPlayer;
