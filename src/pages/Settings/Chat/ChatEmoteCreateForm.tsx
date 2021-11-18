import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatEmoteForm, { ChatEmoteFormData } from "./ChatEmoteForm";
import { toEmoteProps } from "./utils";

const ChatEmoteCreateFormPage: React.FC = () => {
  const { serverId } = useParams<"serverId">();
  const [{ value }] = useCall("chatServer", "listEmotes", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const navigate = useNavigate();
  const [{ error, loading }, createChatEmote] = useLazyCall("chatServer", "createEmote", {
    onComplete: () => navigate(`/settings/chat-servers/${serverId}/emotes`, { replace: true }),
  });

  const onSubmit = (data: ChatEmoteFormData) =>
    createChatEmote({
      serverId: BigInt(serverId),
      ...toEmoteProps(data),
    });

  return (
    <ChatEmoteForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      serverId={BigInt(serverId)}
      indexLinkVisible={!!value?.emotes.length}
    />
  );
};

export default ChatEmoteCreateFormPage;
