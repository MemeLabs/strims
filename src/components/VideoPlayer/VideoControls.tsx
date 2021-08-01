import clsx from "clsx";
import React, { useState } from "react";
import { IconType } from "react-icons/lib";
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
import { RiLayoutRightLine } from "react-icons/ri";
import { useDebounce } from "react-use";

import { VideoControls, VideoState } from "../../hooks/useVideo";
import VideoProgressBar from "./VideoProgressBar";
import VideoVolume from "./VideoVolume";

type ButtonProps = {
  className: string;
  tooltip: string;
  icon: IconType;
  onClick: React.EventHandler<React.UIEvent>;
};

const Button: React.FC<ButtonProps> = ({ className, tooltip, icon: Icon, onClick }) => (
  <div className={clsx("button-wrap", className)}>
    <button data-tip={tooltip} onClick={onClick}>
      <Icon />
    </button>
  </div>
);

type PiPButtonProps = {
  supported: boolean;
  toggle: () => void;
};

const PiPButton: React.FC<PiPButtonProps> = ({ supported, toggle }) =>
  !supported ? null : (
    <Button className="pip" tooltip="Miniplayer" onClick={toggle} icon={MdPictureInPictureAlt} />
  );

type TheaterButtonProps = {
  enabled: boolean;
  toggle: (state: boolean) => void;
};

const TheaterButton: React.FC<TheaterButtonProps> = ({ enabled, toggle }) => (
  <Button
    className="theater"
    tooltip={enabled ? "Exit theater mode" : "Theater mode"}
    onClick={() => toggle(!enabled)}
    icon={RiLayoutRightLine}
  />
);

type FullscreenButtonProps = {
  supported: boolean;
  enabled: boolean;
  toggle: () => void;
};

const FullscreenButton: React.FC<FullscreenButtonProps> = ({ supported, enabled, toggle }) =>
  !supported ? null : (
    <Button
      className="fullscreen"
      tooltip={enabled ? "Exit full screen" : "Full screen"}
      onClick={toggle}
      icon={enabled ? MdFullscreenExit : MdFullscreen}
    />
  );

type VolumeControlProps = {
  volume: number;
  videoControls: VideoControls;
  onUpdateStart: () => void;
  onUpdateEnd: () => void;
};

const VolumeControl: React.FC<VolumeControlProps> = ({
  volume,
  videoControls,
  onUpdateStart,
  onUpdateEnd,
}) => {
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

type VideoControlsProps = {
  visible: boolean;
  fullscreen: boolean;
  toggleFullscreen: () => void;
  theaterMode: boolean;
  toggleTheaterMode: (state: boolean) => void;
  videoState: VideoState;
  videoControls: VideoControls;
};

const VideoControls: React.FC<VideoControlsProps> = (props) => {
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
          tooltip={playing ? "Pause" : "Play"}
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
        <TheaterButton enabled={props.theaterMode} toggle={props.toggleTheaterMode} />
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
