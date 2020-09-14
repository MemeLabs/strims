import React, { FunctionComponent, useEffect, useRef } from "react";
import { MdLoop } from "react-icons/md";
import useFullscreen from "use-fullscreen";

import useIdleTimeout from "../../hooks/useIdleTimeout";
import useMediaSource, { MediaSourceProps } from "../../hooks/useMediaSource";
import useVideo from "../../hooks/useVideo";
import LogoButton from "./LogoButton";
import VideoControls from "./VideoControls";

interface SwarmPlayerProps extends MediaSourceProps {
  volumeStepSize?: number;
}

const SwarmPlayer: FunctionComponent<SwarmPlayerProps> = ({
  networkKey,
  swarmKey,
  mimeType,
  volumeStepSize = 0.1,
}) => {
  const rootRef = useRef();
  const [controlsHidden, renewControlsTimeout, clearControlsTimeout] = useIdleTimeout();
  const [isFullscreen, toggleFullscreen] = useFullscreen();
  const [videoState, videoProps, videoControls] = useVideo();
  const mediaSource = useMediaSource({ networkKey, swarmKey, mimeType, videoRef: videoProps.ref });

  useEffect(() => {
    videoControls.setSrc(URL.createObjectURL(mediaSource));
  }, [videoProps.ref, mediaSource]);

  const waitingSpinner =
    videoState.waiting && videoState.loaded ? (
      <div className="video_player__waiting_spinner">
        <MdLoop />
      </div>
    ) : (
      <LogoButton
        visible={!videoState.playing && !videoState.paused}
        onClick={videoControls.play}
        flicker={videoState.ended && !videoState.loaded}
        spin={videoState.waiting && videoState.loaded}
        disabled={videoState.waiting || !videoState.loaded}
        blur={true}
      />
    );

  const handleToggleFullscreen = () => toggleFullscreen(rootRef.current);

  const handleWheel = React.useCallback(
    (e) => {
      const direction = e.deltaY < 0 ? 1 : -1;
      videoControls.setVolume(videoState.volume + direction * volumeStepSize);
      renewControlsTimeout();
    },
    [videoState.volume, volumeStepSize]
  );

  return (
    <div
      className="video_player"
      onMouseMove={renewControlsTimeout}
      onMouseLeave={clearControlsTimeout}
      onDoubleClick={handleToggleFullscreen}
      onWheel={handleWheel}
      ref={rootRef}
    >
      <video onClick={(e) => e.preventDefault()} className="video_player__video" {...videoProps} />
      {waitingSpinner}
      <VideoControls
        videoState={videoState}
        videoControls={videoControls}
        visible={!controlsHidden}
        fullscreen={isFullscreen}
        toggleFullscreen={handleToggleFullscreen}
      />
    </div>
  );
};

export default SwarmPlayer;
