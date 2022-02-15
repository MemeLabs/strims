import "./ThingTable.scss";

import React from "react";
import { BsThreeDots } from "react-icons/bs";
import { Link, Navigate } from "react-router-dom";

import { Network } from "../../../apis/strims/network/v1/network";
import Dropdown from "../../../components/Dropdown";
import { useCall, useClient, useLazyCall } from "../../../contexts/FrontendApi";
import { useSession } from "../../../contexts/Session";
import { certificateRoot } from "../../../lib/certificate";
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
  const [{ profile }] = useSession();

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
      <tr className="thing_list__item" key={network.id.toString()}>
        <td>
          <Link to={network.id.toString()}>
            {certificateRoot(network.certificate).subject || "unknown"}
          </Link>
        </td>
        <td>{new Date(Number(network.certificate.notAfter) * 1000).toLocaleString()}</td>
        <td className="thing_table__controls">
          <Dropdown
            baseClassName="thing_table_item_dropdown"
            anchor={<BsThreeDots />}
            items={
              <>
                <button className="thing_table_item_dropdown__button" onClick={handleDelete}>
                  delete
                </button>
                <button onClick={handleCreateInvite} className="thing_table_item_dropdown__button">
                  create invite
                </button>
                <button onClick={handlePublish} className="thing_table_item_dropdown__button">
                  publish
                </button>
              </>
            }
          />
        </td>
      </tr>
    );
  });

  const modal = publishNetwork && (
    <PublishNetworkModal network={publishNetwork} onClose={() => setPublishNetwork(null)} />
  );

  return (
    <>
      {modal}
      <table className="thing_table">
        <thead>
          <tr>
            <th>Name</th>
            <th>Certificate expires</th>
            <th></th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </table>
    </>
  );
};

const ChatServerList: React.FC = () => {
  const [{ loading, value }, getServers] = useCall("network", "list");

  if (loading) {
    return null;
  }
  if (!value?.networks.length) {
    return <Navigate to="/settings/networks/new" replace />;
  }
  return (
    <>
      <div className="thing_table__menu">
        <Dropdown
          baseClassName="thing_table_dropdown"
          anchor={"Add network"}
          items={
            <>
              <Link to="/settings/networks/new" className="thing_table_dropdown__button">
                Create new network
              </Link>
              <Link to="/settings/networks/join" className="thing_table_dropdown__button">
                Add invitation code
              </Link>
            </>
          }
        />
      </div>
      <ChatServerTable networks={value.networks} onDelete={() => getServers()} />
    </>
  );
};

export default ChatServerList;
