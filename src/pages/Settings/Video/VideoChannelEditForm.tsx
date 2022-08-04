// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { VideoChannel } from "../../../apis/strims/video/v1/channel";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import VideoChannelForm, { VideoChannelFormData, themeColorOptions } from "./VideoChannelForm";

const VideoChannelEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.videoChannel.title"));

  const { channelId } = useParams<"channelId">();
  const [{ value, ...getRes }] = useCall("videoChannel", "get", {
    args: [{ id: BigInt(channelId) }],
  });

  const navigate = useNavigate();
  const [updateRes, updateVideoChannel] = useLazyCall("videoChannel", "update", {
    onComplete: () => navigate(`/settings/video/channels`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: VideoChannelFormData) => {
    await updateVideoChannel({
      id: BigInt(channelId),
      directoryListingSnippet: {
        title: data.title,
        description: data.description,
        tags: data.tags.map(({ value }) => value),
        themeColor: data.themeColor?.value,
      },
      networkKey: Base64.toUint8Array(data.networkKey),
    });
  }, []);

  if (getRes.loading) {
    return null;
  }

  const { channel } = value;
  const data: VideoChannelFormData = {
    title: channel.directoryListingSnippet.title,
    description: channel.directoryListingSnippet.description,
    tags: channel.directoryListingSnippet.tags.map((value) => ({ label: value, value })),
    networkKey: "",
    themeColor: themeColorOptions.find(
      ({ value }) => value === channel.directoryListingSnippet.themeColor
    ),
  };

  switch (channel.owner.case) {
    case VideoChannel.OwnerCase.LOCAL:
      data.networkKey = Base64.fromUint8Array(channel.owner.local.networkKey);
      break;
    case VideoChannel.OwnerCase.LOCAL_SHARE:
      break;
    case VideoChannel.OwnerCase.REMOTE_SHARE:
      break;
  }

  return (
    <>
      <TableTitleBar label="Edit Channel" backLink="/settings/video/channels" />
      <VideoChannelForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={data}
        submitLabel="Update Channel"
      />
    </>
  );
};

export default VideoChannelEditForm;
