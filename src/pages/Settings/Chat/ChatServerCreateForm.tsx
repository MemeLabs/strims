// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatServerForm, { ChatServerFormData } from "./ChatServerForm";

const ChatServerCreateFormPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const [{ value }] = useCall("chatServer", "listServers");
  const navigate = useNavigate();
  const [{ error, loading }, createChatServer] = useLazyCall("chatServer", "createServer", {
    onComplete: (res) => navigate(`/settings/chat-servers/${res.server.id}`, { replace: true }),
  });

  const onSubmit = (data: ChatServerFormData) =>
    createChatServer({
      networkKey: Base64.toUint8Array(data.networkKey),
      room: {
        name: data.name,
      },
    });

  return (
    <>
      <TableTitleBar
        label="Create Server"
        backLink={!!value?.servers.length && "/settings/chat-servers"}
      />
      <ChatServerForm
        onSubmit={onSubmit}
        error={error}
        loading={loading}
        submitLabel="Create Server"
      />
    </>
  );
};

export default ChatServerCreateFormPage;
