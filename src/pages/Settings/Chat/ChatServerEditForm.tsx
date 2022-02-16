import { Base64 } from "js-base64";
import React from "react";
import { useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatServerForm, { ChatServerFormData } from "./ChatServerForm";

const ChatServerEditForm: React.FC = () => {
  const { serverId } = useParams<"serverId">();
  const [{ value, ...getRes }] = useCall("chatServer", "getServer", {
    args: [{ id: BigInt(serverId) }],
  });

  const [updateRes, updateChatServer] = useLazyCall("chatServer", "updateServer");

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
    <ChatServerForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      id={BigInt(serverId)}
      values={data}
      indexLinkVisible={true}
    />
  );
};

export default ChatServerEditForm;
