import React from "react";
import { Link, Redirect } from "react-router-dom";

import { Network } from "../../../apis/strims/network/v1/network";
import { useCall, useClient, useLazyCall } from "../../../contexts/FrontendApi";
import { useProfile } from "../../../contexts/Profile";
import { certificateRoot } from "../../../lib/certificate";
import jsonutil from "../../../lib/jsonutil";
import PublishNetworkModal from "./PublishNetworkModal";

interface ChatServerTableProps {
  networks: Network[];
  onDelete: () => void;
}

const ChatServerTable: React.FC<ChatServerTableProps> = ({ networks, onDelete }) => {
  const [{ error }, deleteChatServer] = useLazyCall("network", "delete", {
    onComplete: onDelete,
  });
  const client = useClient();
  const [{ profile }] = useProfile();

  const [publishNetwork, setPublishNetwork] = React.useState<Network>();

  if (!networks) {
    return null;
  }

  const rows = networks.map((network) => {
    const handleDelete = () => deleteChatServer({ id: network.id });

    const handleCreateInvite = async () => {
      const invitation = await client.network.createInvitation({
        signingKey: profile.key,
        signingCert: network.certificate,
        networkName: certificateRoot(network.certificate).subject,
      });
      void navigator.clipboard.writeText(invitation.invitationB64);
      console.log("copied invite to clipboard");
    };

    const handlePublish = () => setPublishNetwork(network);

    return (
      <div className="thing_list__item" key={network.id.toString()}>
        <Link to={`/settings/networks/${network.id}`}>
          {certificateRoot(network.certificate).subject || "unknown"}
        </Link>
        <button className="input input_button" onClick={handleDelete}>
          delete
        </button>
        <button onClick={handleCreateInvite} className="input input_button">
          create invite
        </button>
        <button onClick={handlePublish} className="input input_button">
          publish
        </button>
        <pre>{jsonutil.stringify(network)}</pre>
      </div>
    );
  });

  const modal = publishNetwork && (
    <PublishNetworkModal network={publishNetwork} onClose={() => setPublishNetwork(null)} />
  );

  return (
    <div className="thing_list">
      {modal}
      {rows}
    </div>
  );
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
