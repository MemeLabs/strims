// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatServerForm, { ChatServerFormData } from "./ChatServerForm";

const ChatServerEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const { serverId } = useParams<"serverId">();
  const [{ value, ...getRes }] = useCall("chatServer", "getServer", {
    args: [{ id: BigInt(serverId) }],
  });

  const navigate = useNavigate();
  const [updateRes, updateChatServer] = useLazyCall("chatServer", "updateServer", {
    onComplete: () => navigate("/settings/chat-servers"),
  });

  const onSubmit = (data: ChatServerFormData) =>
    updateChatServer({
      id: BigInt(serverId),
      networkKey: Base64.toUint8Array(data.networkKey),
      room: {
        name: data.name,
      },
    });

  if (getRes.loading) {
    return null;
  }

  const { server } = value;
  const data: ChatServerFormData = {
    name: server.room.name,
    networkKey: Base64.fromUint8Array(server.networkKey),
  };

  return (
    <>
      <TableTitleBar label="Edit Server" backLink="/settings/chat-servers" />
      <ChatServerForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        id={BigInt(serverId)}
        values={data}
        submitLabel="Update Server"
      />
    </>
  );
};

export default ChatServerEditForm;
