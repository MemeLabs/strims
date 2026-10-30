// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoPlayer.scss";

import clsx from "clsx";
import React, { useCallback, useEffect, useRef } from "react";
import { MdLoop } from "react-icons/md";
import useFullscreen from "use-fullscreen";

import { useLayout } from "../../contexts/Layout";
import useIdleTimeout from "../../hooks/useIdleTimeout";
import { MediaSourceProps } from "../../hooks/useMediaSource";
import useVideo from "../../hooks/useVideo";
import { StreamFormat, streamFormat } from "../../lib/media/mime";
import { DEVICE_TYPE, DeviceType } from "../../lib/userAgent";
import { MediaRelayMedia, MediaSourceMedia, UnitsMedia } from "./SwarmMedia";
import VideoControls from "./VideoControls";

interface SwarmPlayerProps extends Pick<MediaSourceProps, "networkKey" | "swarmUri" | "mimeType"> {
  defaultControlsVisible?: boolean;
  disableControls?: boolean;
  defaultAspectRatio?: string | number;
  handleClose: () => void;
  className?: string;
}

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

  // Unit-format streams play through the player worker everywhere; record
  // streams through a window MediaSource, or the service worker's HLS relay
  // where the window has none (iOS).
  const format = streamFormat(mimeType);
  const SwarmMedia =
    format === StreamFormat.Units
      ? UnitsMedia
      : window.MediaSource
      ? MediaSourceMedia
      : MediaRelayMedia;

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
      <SwarmMedia
        key={`${format}:${networkKey}:${swarmUri}`}
        networkKey={networkKey}
        swarmUri={swarmUri}
        mimeType={mimeType}
        videoRef={videoRef}
        videoControls={videoControls}
      />
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
        {...controls}
      />
    </div>
  );
};

export default SwarmPlayer;
