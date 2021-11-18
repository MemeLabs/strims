import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatModifierForm, { ChatModifierFormData } from "./ChatModifierForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const { serverId } = useParams<"serverId">();
  const [{ value }] = useCall("chatServer", "listModifiers", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const navigate = useNavigate();
  const [{ error, loading }, createChatModifier] = useLazyCall("chatServer", "createModifier", {
    onComplete: () => navigate(`/settings/chat-servers/${serverId}/modifiers`, { replace: true }),
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
