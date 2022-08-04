// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatTagForm, { ChatTagFormData } from "./ChatTagForm";

const ChatTagEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const { serverId, tagId } = useParams<"serverId" | "tagId">();
  const [getRes] = useCall("chatServer", "getTag", { args: [{ id: BigInt(tagId) }] });

  const [updateRes, updateChatTag] = useLazyCall("chatServer", "updateTag");

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
    <>
      <TableTitleBar label="Edit Tag" backLink={`/settings/chat-servers/${serverId}/tags`} />
      <ChatTagForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={getRes.value?.tag}
        submitLabel="Update Tag"
      />
    </>
  );
};

export default ChatTagEditForm;
