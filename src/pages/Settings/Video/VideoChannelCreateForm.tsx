// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useNavigate } from "react-router-dom";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import VideoChannelForm, { VideoChannelFormData } from "./VideoChannelForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const [{ value }] = useCall("videoChannel", "list");
  const navigate = useNavigate();
  const [{ error, loading }, createChannel] = useLazyCall("videoChannel", "create", {
    onComplete: () => navigate(`/settings/video/channels`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: VideoChannelFormData) => {
    await createChannel({
      directoryListingSnippet: {
        title: data.title,
        description: data.description,
        tags: data.tags.map(({ value }) => value),
        themeColor: data.themeColor?.value,
      },
      networkKey: Base64.toUint8Array(data.networkKey),
    });
  }, []);

  const backLink = value?.channels.length ? `/settings/video/channels` : `/settings/video/ingress`;

  return (
    <>
      <TableTitleBar label="Create Channel" backLink={backLink} />
      <VideoChannelForm
        onSubmit={onSubmit}
        error={error}
        loading={loading}
        submitLabel="Create Channel"
      />
    </>
  );
};

export default ChatModifierCreateFormPage;
