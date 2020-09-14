import clsx from "clsx";
import React, { useState } from "react";
import {
  MdFullscreen,
  MdFullscreenExit,
  MdPause,
  MdPictureInPictureAlt,
  MdPlayArrow,
  MdVolumeDown,
  MdVolumeMute,
  MdVolumeOff,
  MdVolumeUp,
} from "react-icons/md";
import { useDebounce } from "react-use";

import VideoProgressBar from "./VideoProgressBar";
import VideoVolume from "./VideoVolume";

const Button = ({ className, tooltip, icon: Icon, onClick }) => (
  <div className={clsx("button-wrap", className)}>
    <button data-tip={tooltip} onClick={onClick}>
      <Icon />
    </button>
  </div>
);

const PiPButton = ({ supported, toggle }) =>
  !supported ? null : (
    <Button className="pip" tooltip="Miniplayer" onClick={toggle} icon={MdPictureInPictureAlt} />
  );

const FullscreenButton = ({ supported, enabled, toggle }) =>
  !supported ? null : (
    <Button
      className="fullscreen"
      tooltip={enabled ? "Exit full screen" : "Full screen"}
      onClick={toggle}
      icon={enabled ? MdFullscreenExit : MdFullscreen}
    />
  );

const VolumeControl = ({ volume, videoControls, onUpdateStart, onUpdateEnd }) => {
  const volumeIcons = [MdVolumeOff, MdVolumeMute, MdVolumeDown, MdVolumeUp];
  const volumeLevel = Math.ceil(volume * (volumeIcons.length - 1));
  const VolumeIcon = volumeIcons[volumeLevel];
  const handleVolumeClick = () => (volume === 0 ? videoControls.unmute() : videoControls.mute());

  return (
    <div className="volume button-wrap">
      <button data-tip={volume === 0 ? "Unmute" : "Mute"} onClick={handleVolumeClick}>
        <VolumeIcon className={`volume-level-${volumeLevel}`} />
      </button>
      <VideoVolume
        onUpdate={videoControls.setVolume}
        onSlideStart={onUpdateStart}
        onSlideEnd={onUpdateEnd}
        value={volume}
      />
    </div>
  );
};

const VideoControls = (props) => {
  const [active, setActive] = useState(false);
  const visible = props.visible || active;

  const [visible100, setVisible100] = useState(false);
  const [visible500, setVisible500] = useState(false);
  useDebounce(() => setVisible100(visible), 100, [visible]);
  useDebounce(() => setVisible500(visible), 500, [visible]);

  if (!visible && !visible500) {
    return null;
  }

  const { videoState, videoControls } = props;

  const { playing } = videoState;

  const controlsClassName = clsx({
    video_player__controls: true,
    visible,
    visible100,
    visible500,
  });

  return (
    <div
      className={controlsClassName}
      onMouseMove={() => setActive(true)}
      onMouseLeave={() => setActive(false)}
    >
      <div className="controls_group left">
        <Button
          className="play"
          tooltip={playing === 0 ? "Pause" : "Play"}
          onClick={playing ? videoControls.pause : videoControls.play}
          icon={playing ? MdPause : MdPlayArrow}
        />
        <VolumeControl
          volume={videoState.volume}
          videoControls={videoControls}
          onUpdateStart={() => setActive(true)}
          onUpdateEnd={() => setActive(false)}
        />
      </div>
      <div className="progress_bar">
        <VideoProgressBar videoState={videoState} videoControls={videoControls} />
      </div>
      <div className="controls_group right">
        <PiPButton supported={videoState.supportPiP} toggle={videoControls.togglePiP} />
        <FullscreenButton
          supported={document.fullscreenEnabled}
          enabled={props.fullscreen}
          toggle={props.toggleFullscreen}
        />
      </div>
    </div>
  );
};

export default VideoControls;
