// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoMeta.scss";

import React, { useContext } from "react";
import { HiOutlineUser } from "react-icons/hi";
import { TbArrowAutofitHeight, TbArrowAutofitUp } from "react-icons/tb";

import monkey from "../../assets/directory/monkey.png";
import { PlayerContext, PlayerMode } from "../contexts/Player";
import { useListing } from "../hooks/directory";
import { useStableCallback } from "../hooks/useStableCallback";
import { formatNumber } from "../lib/number";
import SnippetImage from "./Directory/SnippetImage";
import Stopwatch from "./Stopwatch";

const VideoMeta: React.FC = () => {
  const { listingId: listingId } = useContext(PlayerContext);
  return listingId && <VideoMetaContent />;
};

export default VideoMeta;

const VideoMetaContent: React.FC = () => {
  const { networkKey, listingId } = useContext(PlayerContext);
  const { listing } = useListing(networkKey, listingId);

  if (!listing) {
    return null;
  }

  const { snippet } = listing;
  const title = snippet?.title;

  return (
    <div>
      <div className="video_meta">
        <SnippetImage
          className="video_meta__logo"
          fallback={monkey}
          source={snippet?.channelLogo}
        />
        <div className="video_meta__label">
          {title && (
            <span className="video_meta__title" title={title}>
              {title}
            </span>
          )}
          {snippet?.channelName && <span className="video_meta__name">{snippet?.channelName}</span>}
        </div>
        <div className="video_meta__stats">
          <span
            className="video_meta__viewers"
            title={`${formatNumber(Number(snippet?.userCount))} ${
              snippet?.live ? "viewers" : "views"
            }`}
          >
            <HiOutlineUser />
            {formatNumber(listing.userCount)}
          </span>
          {snippet?.startTime && (
            <span className="video_meta__runtime">
              <Stopwatch startTime={Number(snippet?.startTime)} />
            </span>
          )}
          <HeightToggleButton />
        </div>
      </div>
    </div>
  );
};

const HeightToggleButton: React.FC = () => {
  const { mode, setMode } = useContext(PlayerContext);

  const handleToggleClick = useStableCallback(() => {
    setMode(mode === PlayerMode.FULL ? PlayerMode.LARGE : PlayerMode.FULL);
  });

  return (
    <button
      className="video_meta__toggle_height"
      onClick={handleToggleClick}
      title="Toggle player height"
    >
      {mode === PlayerMode.FULL ? <TbArrowAutofitUp /> : <TbArrowAutofitHeight />}
    </button>
  );
};
