import React from "react";
import { useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatModifierForm, { ChatModifierFormData } from "./ChatModifierForm";

const ChatModifierEditForm: React.FC = () => {
  const { serverId, modifierId } = useParams<"serverId" | "modifierId">();
  const [getRes] = useCall("chatServer", "getModifier", { args: [{ id: BigInt(modifierId) }] });

  const [updateRes, updateChatModifier] = useLazyCall("chatServer", "updateModifier");

  const onSubmit = (data: ChatModifierFormData) =>
    updateChatModifier({
      id: BigInt(modifierId),
      serverId: BigInt(serverId),
      ...data,
    });

  if (getRes.loading) {
    return null;
  }

  return (
    <ChatModifierForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      values={getRes.value?.modifier}
      serverId={BigInt(serverId)}
      indexLinkVisible={true}
    />
  );
};

export default ChatModifierEditForm;
