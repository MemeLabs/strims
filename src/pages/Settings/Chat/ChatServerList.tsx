import React from "react";
import { Link, Redirect } from "react-router-dom";

import { Server } from "../../../apis/strims/chat/v1/chat";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import jsonutil from "../../../lib/jsonutil";

interface ChatServerTableProps {
  servers: Server[];
  onDelete: () => void;
}

const ChatServerTable: React.FC<ChatServerTableProps> = ({ servers, onDelete }) => {
  const [{ error }, deleteChatServer] = useLazyCall("chat", "deleteServer", {
    onComplete: onDelete,
  });

  if (!servers) {
    return null;
  }

  const rows = servers.map((server) => {
    const handleDelete = () => deleteChatServer({ id: server.id });

    return (
      <div className="thing_list__item" key={server.id.toString()}>
        <Link to={`/settings/chat-servers/${server.id}`}>{server.room.name || "no title"}</Link>
        <button className="input input_button" onClick={handleDelete}>
          delete
        </button>
        <pre>{jsonutil.stringify(server)}</pre>
      </div>
    );
  });
  return <div className="thing_list">{rows}</div>;
};

const ChatServerList: React.FC = () => {
  const [{ loading, value }, getServers] = useCall("chat", "listServers");

  if (loading) {
    return null;
  }
  if (!value?.servers.length) {
    return <Redirect to="/settings/chat-servers/new" />;
  }
  return (
    <>
      <ChatServerTable servers={value.servers} onDelete={() => getServers()} />
      <Link to="/settings/chat-servers/new">Create Server</Link>
    </>
  );
};

export default ChatServerList;
