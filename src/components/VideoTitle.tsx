// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoMeta.scss";

import React, { useContext } from "react";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { PlayerContext } from "../contexts/Player";
import { useListing } from "../hooks/directory";

const VideoTitle: React.FC = () => {
  const { t } = useTranslation();
  const { networkKey, listingId } = useContext(PlayerContext);
  const { listing } = useListing(networkKey, listingId);

  const title = [listing?.snippet?.title, listing?.snippet?.channelName, t("title")];
  useTitle(title.filter(Boolean).join(" - "));

  return null;
};

export default VideoTitle;
