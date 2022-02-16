import { Base64 } from "js-base64";
import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { VideoChannel } from "../../../apis/strims/video/v1/channel";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import VideoChannelForm, { VideoChannelFormData } from "./VideoChannelForm";

const VideoChannelEditForm: React.FC = () => {
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
    <VideoChannelForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      values={data}
      indexLinkVisible={true}
    />
  );
};

export default VideoChannelEditForm;
