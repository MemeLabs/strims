import React from "react";
import { useHistory, useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatEmoteForm, { ChatEmoteFormData } from "./ChatEmoteForm";
import { toEmoteProps } from "./utils";

const ChatEmoteCreateFormPage: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [{ value }] = useCall("chatServer", "listEmotes", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const history = useHistory();
  const [{ error, loading }, createChatEmote] = useLazyCall("chatServer", "createEmote", {
    onComplete: () => history.replace(`/settings/chat-servers/${serverId}/emotes`),
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
