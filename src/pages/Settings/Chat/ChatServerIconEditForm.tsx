// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { useTitle } from "react-use";

import { ImageValue } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { fromFormImageValue, toFormImageValue } from "../../../lib/image";
import ChatServerIconForm, { ChatServerIconFormData } from "./ChatServerIconForm";

export interface ChatServerIconEditFormData {
  image: ImageValue;
}

const ChatServerIconEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const { serverId } = useParams<"serverId">();
  const [{ value, ...getRes }] = useCall("chatServer", "getServerIcon", {
    args: [{ serverId: BigInt(serverId) }],
  });
  const navigate = useNavigate();
  const [updateRes, updateServerIcon] = useLazyCall("chatServer", "updateServerIcon", {
    onComplete: () => navigate(`/settings/chat-servers/${serverId}`),
  });

  const onSubmit = (data: ChatServerIconFormData) =>
    updateServerIcon({
      serverId: BigInt(serverId),
      image: fromFormImageValue(data.image),
    });

  if (getRes.loading) {
    return null;
  }

  const data: ChatServerIconFormData = {
    image: value.serverIcon ? toFormImageValue(value.serverIcon.image) : null,
  };

  return (
    <>
      <TableTitleBar label="Edit Server Icon" backLink={`/settings/chat-servers/${serverId}`} />
      <ChatServerIconForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={data}
        submitLabel={"Update Server Icon"}
      />
    </>
  );
};

export default ChatServerIconEditForm;
