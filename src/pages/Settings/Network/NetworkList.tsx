import React from "react";
import { Link, Redirect } from "react-router-dom";

import { Network } from "../../../apis/strims/network/v1/network";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { certificateRoot } from "../../../lib/certificate";
import jsonutil from "../../../lib/jsonutil";

interface ChatServerTableProps {
  networks: Network[];
  onDelete: () => void;
}

const ChatServerTable: React.FC<ChatServerTableProps> = ({ networks, onDelete }) => {
  const [{ error }, deleteChatServer] = useLazyCall("network", "delete", {
    onComplete: onDelete,
  });

  if (!networks) {
    return null;
  }

  const rows = networks.map((network) => {
    const handleDelete = () => deleteChatServer({ id: network.id });

    return (
      <div className="thing_list__item" key={network.id.toString()}>
        <Link to={`/settings/networks/${network.id}`}>
          {certificateRoot(network.certificate).subject || "unknown"}
        </Link>
        <button className="input input_button" onClick={handleDelete}>
          delete
        </button>
        <pre>{jsonutil.stringify(network)}</pre>
      </div>
    );
  });
  return <div className="thing_list">{rows}</div>;
};

const ChatServerList: React.FC = () => {
  const [{ loading, value }, getServers] = useCall("network", "list");

  if (loading) {
    return null;
  }
  if (!value?.networks.length) {
    return <Redirect to="/settings/networks/new" />;
  }
  return (
    <>
      <ChatServerTable networks={value.networks} onDelete={() => getServers()} />
      <Link to="/settings/networks/new">Create Server</Link>
    </>
  );
};

export default ChatServerList;
