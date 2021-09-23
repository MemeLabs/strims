import React from "react";
import { useHistory, useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatModifierForm, { ChatModifierFormData } from "./ChatModifierForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [{ value }] = useCall("chatServer", "listModifiers", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const history = useHistory();
  const [{ error, loading }, createChatModifier] = useLazyCall("chatServer", "createModifier", {
    onComplete: () => history.replace(`/settings/chat-servers/${serverId}/modifiers`),
  });

  const onSubmit = (data: ChatModifierFormData) =>
    createChatModifier({
      serverId: BigInt(serverId),
      ...data,
    });

  return (
    <ChatModifierForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      serverId={BigInt(serverId)}
      indexLinkVisible={!!value?.modifiers.length}
    />
  );
};

export default ChatModifierCreateFormPage;
