import { Base64 } from "js-base64";
import React from "react";
import { useNavigate } from "react-router-dom";

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
      },
      networkKey: Base64.toUint8Array(data.networkKey),
    });
  }, []);

  return (
    <VideoChannelForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      indexLinkVisible={!!value?.channels.length}
    />
  );
};

export default ChatModifierCreateFormPage;
