import React from "react";
import { useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatTagForm, { ChatTagFormData } from "./ChatTagForm";

const ChatTagEditForm: React.FC = () => {
  const { serverId, tagId } = useParams<{ serverId: string; tagId: string }>();
  const [getRes] = useCall("chat", "getTag", { args: [{ id: BigInt(tagId) }] });

  const [updateRes, updateChatTag] = useLazyCall("chat", "updateTag");

  const onSubmit = (data: ChatTagFormData) =>
    updateChatTag({
      id: BigInt(tagId),
      serverId: BigInt(serverId),
      ...data,
    });

  if (getRes.loading) {
    return null;
  }

  return (
    <ChatTagForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      values={getRes.value?.tag}
      serverId={BigInt(serverId)}
      indexLinkVisible={true}
    />
  );
};

export default ChatTagEditForm;
