// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatEmoteForm, { ChatEmoteFormData } from "./ChatEmoteForm";
import { toEmoteProps } from "./utils";

const ChatEmoteCreateFormPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const { serverId } = useParams<"serverId">();
  const [{ value }] = useCall("chatServer", "listEmotes", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const navigate = useNavigate();
  const [{ error, loading }, createChatEmote] = useLazyCall("chatServer", "createEmote", {
    onComplete: () => navigate(`/settings/chat-servers/${serverId}/emotes`, { replace: true }),
  });

  const onSubmit = async (data: ChatEmoteFormData) =>
    await createChatEmote({
      serverId: BigInt(serverId),
      ...(await toEmoteProps(data)),
    });

  const backLink = value?.emotes.length
    ? `/settings/chat-servers/${serverId}/emotes`
    : `/settings/chat-servers/${serverId}`;

  return (
    <>
      <TableTitleBar label="Create Emote" backLink={backLink} />
      <ChatEmoteForm
        onSubmit={onSubmit}
        error={error}
        loading={loading}
        serverId={BigInt(serverId)}
        submitLabel="Create Emote"
      />
    </>
  );
};

export default ChatEmoteCreateFormPage;
