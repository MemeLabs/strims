import React from "react";
import { useHistory, useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatTagForm, { ChatTagFormData } from "./ChatTagForm";

const ChatTagCreateFormPage: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [{ value }] = useCall("chatServer", "listTags", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const history = useHistory();
  const [{ error, loading }, createChatTag] = useLazyCall("chatServer", "createTag", {
    onComplete: (res) => history.replace(`/settings/chat-servers/${serverId}/tags`),
  });

  const onSubmit = (data: ChatTagFormData) =>
    createChatTag({
      serverId: BigInt(serverId),
      ...data,
    });

  return (
    <ChatTagForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      serverId={BigInt(serverId)}
      indexLinkVisible={!!value?.tags.length}
    />
  );
};

export default ChatTagCreateFormPage;
