import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import VideoChannelForm, { VideoChannelFormData } from "./VideoChannelForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const { serverId } = useParams<"serverId">();
  const [{ value }] = useCall("videoChannel", "list", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const navigate = useNavigate();
  const [{ error, loading }, createChatModifier] = useLazyCall("videoChannel", "create", {
    onComplete: () => navigate(`/settings/video-ingress/channels`, { replace: true }),
  });

  const onSubmit = (data: VideoChannelFormData) =>
    createChatModifier({
      serverId: BigInt(serverId),
      ...data,
    });

  return (
    <VideoChannelForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      serverId={BigInt(serverId)}
      indexLinkVisible={!!value?.modifiers.length}
    />
  );
};

export default ChatModifierCreateFormPage;
