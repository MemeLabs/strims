import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatTagForm, { ChatTagFormData } from "./ChatTagForm";

const ChatTagCreateFormPage: React.FC = () => {
  const { serverId } = useParams<"serverId">();
  const [{ value }] = useCall("chatServer", "listTags", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const navigate = useNavigate();
  const [{ error, loading }, createChatTag] = useLazyCall("chatServer", "createTag", {
    onComplete: () => navigate(`/settings/chat-servers/${serverId}/tags`, { replace: true }),
  });

  const onSubmit = (data: ChatTagFormData) =>
    createChatTag({
      serverId: BigInt(serverId),
      ...data,
    });

  const backLink = value?.tags.length
    ? `/settings/chat-servers/${serverId}/tags`
    : `/settings/chat-servers/${serverId}`;

  return (
    <>
      <TableTitleBar label="Create Tag" backLink={backLink} />
      <ChatTagForm onSubmit={onSubmit} error={error} loading={loading} submitLabel="Create Tag" />
    </>
  );
};

export default ChatTagCreateFormPage;
