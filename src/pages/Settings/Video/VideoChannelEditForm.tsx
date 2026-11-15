// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { useTitle } from "react-use";

import { ListingSnippetImage_SourceOneofCase } from "../../../apis/strims/network/v1/directory/directory.pb";
import { VideoChannel_OwnerCase } from "../../../apis/strims/video/v1/channel.pb";
import { InputError } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { toFileType, toImageType } from "../../../lib/image";
import { formatError } from "../../../lib/rpcError";
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
    onComplete: () => navigate(`/settings/video/channels`),
  });

  const onSubmit = React.useCallback(async (data: VideoChannelFormData) => {
    try {
      await updateVideoChannel({
        id: BigInt(channelId),
        directoryListingSnippet: {
          channelName: data.channelName,
          channelLogo: {
            sourceOneof: {
              image: {
                data: Base64.toUint8Array(data.channelLogo.data),
                type: toImageType(data.channelLogo.type),
                height: data.channelLogo.height,
                width: data.channelLogo.width,
              },
            },
          },
          title: data.title,
          description: data.description,
          tags: data.tags.map(({ value }) => value),
          themeColor: data.themeColor?.value,
        },
        networkKey: Base64.toUint8Array(data.networkKey),
      });
    } catch {
      // The form shows updateRes.error.
    }
  }, []);

  if (getRes.loading) {
    return null;
  }
  if (getRes.error) {
    return (
      <>
        <TableTitleBar label="Edit Channel" backLink="/settings/video/channels" />
        <div className="thing_form">
          <InputError error={formatError(getRes.error) || "Error loading channel"} />
        </div>
      </>
    );
  }

  const { channel } = value;
  const snippet = channel.directoryListingSnippet;
  const logo =
    snippet.channelLogo?.sourceOneof?.case === ListingSnippetImage_SourceOneofCase.IMAGE
      ? snippet.channelLogo.sourceOneof.image
      : null;
  const data: VideoChannelFormData = {
    channelName: snippet.channelName,
    channelLogo: logo
      ? {
          data: Base64.fromUint8Array(logo.data),
          type: toFileType(logo.type),
          height: logo.height,
          width: logo.width,
        }
      : null,
    title: snippet.title,
    description: snippet.description,
    tags: snippet.tags.map((value) => ({ label: value, value })),
    networkKey: "",
    themeColor: themeColorOptions.find(({ value }) => value === snippet.themeColor),
  };

  switch (channel.owner.case) {
    case VideoChannel_OwnerCase.LOCAL:
      data.networkKey = Base64.fromUint8Array(channel.owner.local.networkKey);
      break;
    case VideoChannel_OwnerCase.LOCAL_SHARE:
      break;
    case VideoChannel_OwnerCase.REMOTE_SHARE:
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
