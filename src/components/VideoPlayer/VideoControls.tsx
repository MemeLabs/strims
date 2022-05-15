// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoControls.scss";

import clsx from "clsx";
import React, { useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { IconType } from "react-icons/lib";
import {
  MdClose,
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
import { RiLayout6Line, RiLayoutRightLine } from "react-icons/ri";
import { useDebounce } from "react-use";

import useClickAway from "../../hooks/useClickAway";
import { VideoControls, VideoState } from "../../hooks/useVideo";
import VideoProgressBar from "./VideoProgressBar";
import VideoVolume from "./VideoVolume";

interface ButtonProps {
  className: string;
  tooltip: string;
  icon: IconType;
  onClick: React.EventHandler<React.UIEvent>;
}

const Button: React.FC<ButtonProps> = ({ className, tooltip, icon: Icon, onClick }) => (
  <div className={clsx("button-wrap", className)}>
    <button data-tip={tooltip} onClick={onClick}>
      <Icon />
    </button>
  </div>
);

interface PiPButtonProps {
  supported: boolean;
  toggle: () => void;
}

const PiPButton: React.FC<PiPButtonProps> = ({ supported, toggle }) => {
  const { t } = useTranslation();
  return !supported ? null : (
    <Button
      className="pip"
      tooltip={t("player.Miniplayer")}
      onClick={toggle}
      icon={MdPictureInPictureAlt}
    />
  );
};

interface TheaterButtonProps {
  enabled: boolean;
  toggle: (state: boolean) => void;
}

const TheaterButton: React.FC<TheaterButtonProps> = ({ enabled, toggle }) => {
  const { t } = useTranslation();
  return (
    <Button
      className="theater"
      tooltip={enabled ? t("player.Exit theater mode") : t("player.Theater mode")}
      onClick={() => toggle(!enabled)}
      icon={enabled ? RiLayout6Line : RiLayoutRightLine}
    />
  );
};

interface FullscreenButtonProps {
  enabled: boolean;
  toggle: () => void;
}

const FullscreenButton: React.FC<FullscreenButtonProps> = ({ enabled, toggle }) => {
  const { t } = useTranslation();
  return (
    <Button
      className="fullscreen"
      tooltip={enabled ? t("player.Exit full screen") : t("player.Full screen")}
      onClick={toggle}
      icon={enabled ? MdFullscreenExit : MdFullscreen}
    />
  );
};

interface CloseButtonProps {
  onClick: () => void;
}

const CloseButton: React.FC<CloseButtonProps> = ({ onClick }) => {
  const { t } = useTranslation();
  return <Button className="close" tooltip={t("player.Close")} onClick={onClick} icon={MdClose} />;
};

interface VolumeControlProps {
  volume: number;
  videoControls: VideoControls;
  onUpdateStart: () => void;
  onUpdateEnd: () => void;
}

const VolumeControl: React.FC<VolumeControlProps> = ({
  volume,
  videoControls,
  onUpdateStart,
  onUpdateEnd,
}) => {
  const { t } = useTranslation();

  const volumeIcons = [MdVolumeOff, MdVolumeMute, MdVolumeDown, MdVolumeUp];
  const volumeLevel = Math.ceil(volume * (volumeIcons.length - 1));
  const VolumeIcon = volumeIcons[volumeLevel];
  const handleVolumeClick = () => (volume === 0 ? videoControls.unmute() : videoControls.mute());

  return (
    <div className="volume button-wrap">
      <button
        data-tip={volume === 0 ? t("player.Unmute") : t("player.Mute")}
        onClick={handleVolumeClick}
      >
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

interface VideoControlsProps {
  visible: boolean;
  fullscreen: boolean;
  toggleFullscreen: () => void;
  theaterMode: boolean;
  toggleTheaterMode: (state: boolean) => void;
  videoState: VideoState;
  videoControls: VideoControls;
  close: () => void;

  showPlayButton?: boolean;
  showVolume?: boolean;
  showProgressBar?: boolean;
  showPiPButton?: boolean;
  showTheaterButton?: boolean;
  showFullscreenButton?: boolean;
  showCloseButton?: boolean;
}

const VideoControls: React.FC<VideoControlsProps> = (props) => {
  const { t } = useTranslation();

  const [active, setActive] = useState(false);
  const visible = props.visible || active;

  const [visible100, setVisible100] = useState(false);
  const [visible500, setVisible500] = useState(false);
  useDebounce(() => setVisible100(visible), 100, [visible]);
  useDebounce(() => setVisible500(visible), 500, [visible]);

  const ref = useRef<HTMLDivElement>(null);
  useClickAway(ref, () => setActive(false));

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
      ref={ref}
    >
      <div className="controls_group left">
        {props.showPlayButton && (
          <Button
            className="play"
            tooltip={playing ? t("player.Pause") : t("player.Play")}
            onClick={playing ? videoControls.pause : videoControls.play}
            icon={playing ? MdPause : MdPlayArrow}
          />
        )}
        {props.showVolume && (
          <VolumeControl
            volume={videoState.volume}
            videoControls={videoControls}
            onUpdateStart={() => setActive(true)}
            onUpdateEnd={() => setActive(false)}
          />
        )}
      </div>
      <div className="progress_bar">
        {props.showProgressBar && (
          <VideoProgressBar videoState={videoState} videoControls={videoControls} />
        )}
      </div>
      <div className="controls_group right">
        {props.showPiPButton && (
          <PiPButton supported={videoState.supportPiP} toggle={videoControls.togglePiP} />
        )}
        {props.showTheaterButton && (
          <TheaterButton enabled={props.theaterMode} toggle={props.toggleTheaterMode} />
        )}
        {props.showFullscreenButton && (
          <FullscreenButton enabled={props.fullscreen} toggle={props.toggleFullscreen} />
        )}
        {props.showCloseButton && <CloseButton onClick={props.close} />}
      </div>
    </div>
  );
};

export default VideoControls;
