// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatModifierForm, { ChatModifierFormData } from "./ChatModifierForm";

const ChatModifierEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const { serverId, modifierId } = useParams<"serverId" | "modifierId">();
  const [getRes] = useCall("chatServer", "getModifier", { args: [{ id: BigInt(modifierId) }] });

  const navigate = useNavigate();
  const [updateRes, updateChatModifier] = useLazyCall("chatServer", "updateModifier", {
    onComplete: () => navigate(`/settings/chat-servers/${serverId}/modifiers`),
  });

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
    <>
      <TableTitleBar
        label="Edit Modifier"
        backLink={`/settings/chat-servers/${serverId}/modifiers`}
      />
      <ChatModifierForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={getRes.value?.modifier}
        submitLabel="Update Modifier"
      />
    </>
  );
};

export default ChatModifierEditForm;
