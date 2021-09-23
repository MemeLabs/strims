import { Base64 } from "js-base64";
import React from "react";
import { useHistory } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ChatServerForm, { ChatServerFormData } from "./ChatServerForm";

const ChatServerCreateFormPage: React.FC = () => {
  const [{ value }] = useCall("chatServer", "listServers");
  const history = useHistory();
  const [{ error, loading }, createChatServer] = useLazyCall("chatServer", "createServer", {
    onComplete: (res) => history.replace(`/settings/chat-servers/${res.server.id}`),
  });

  const onSubmit = (data: ChatServerFormData) =>
    createChatServer({
      networkKey: Base64.toUint8Array(data.networkKey.value),
      room: {
        name: data.name,
      },
    });

  return (
    <ChatServerForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      indexLinkVisible={!!value?.servers.length}
    />
  );
};

export default ChatServerCreateFormPage;
