import { Base64 } from "js-base64";
import React from "react";
import { useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatServerForm, { ChatServerFormData } from "./ChatServerForm";

const ChatServerEditForm: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [getRes] = useCall("chatServer", "getServer", { args: [{ id: BigInt(serverId) }] });

  const [updateRes, updateChatServer] = useLazyCall("chatServer", "updateServer");

  const onSubmit = (data: ChatServerFormData) =>
    updateChatServer({
      id: BigInt(serverId),
      networkKey: Base64.toUint8Array(data.networkKey.value),
      room: {
        name: data.name,
      },
    });

  return (
    <ChatServerForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      config={getRes.value?.server}
      indexLinkVisible={true}
    />
  );
};

export default ChatServerEditForm;
