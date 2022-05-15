// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { TableTitleBar } from "../../../components/Settings/Table";
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

  const backLink = value?.modifiers.length
    ? `/settings/chat-servers/${serverId}/modifiers`
    : `/settings/chat-servers/${serverId}`;

  return (
    <>
      <TableTitleBar label="Create Modifier" backLink={backLink} />
      <ChatModifierForm
        onSubmit={onSubmit}
        error={error}
        loading={loading}
        submitLabel="Create Modifier"
      />
    </>
  );
};

export default ChatModifierCreateFormPage;
