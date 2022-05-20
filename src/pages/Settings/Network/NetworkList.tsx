// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { Link, useNavigate } from "react-router-dom";

import { Network } from "../../../apis/strims/network/v1/network";
import {
  MenuCell,
  MenuItem,
  MenuLink,
  Table,
  TableMenu,
  TableTitleBar,
} from "../../../components/Settings/Table";
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
    const navigate = useNavigate();
    const handleDelete = () => deleteChatServer({ id: network.id });

    const handleCreateInvite = () => navigate(`${network.id}/invite`);

    const handlePublish = () => setPublishNetwork(network);

    return (
      <tr key={network.id.toString()}>
        <td>
          <Link to={network.id.toString()}>
            {certificateRoot(network.certificate).subject || "unknown"}
          </Link>
        </td>
        <td>{new Date(Number(network.certificate.notAfter) * 1000).toLocaleString()}</td>
        <MenuCell>
          <MenuItem label="Delete" onClick={handleDelete} />
          <MenuItem label="Create Invite" onClick={handleCreateInvite} />
          <MenuItem label="Publish" onClick={handlePublish} />
        </MenuCell>
      </tr>
    );
  });

  const modal = publishNetwork && (
    <PublishNetworkModal network={publishNetwork} onClose={() => setPublishNetwork(null)} />
  );

  return (
    <>
      {modal}
      <Table>
        <thead>
          <tr>
            <th>Name</th>
            <th>Certificate expires</th>
            <th></th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </Table>
    </>
  );
};

const ChatServerList: React.FC = () => {
  const [{ loading, value }, getServers] = useCall("network", "list");

  if (loading) {
    return null;
  }
  // if (!value?.networks.length) {
  //   return <Navigate to="/settings/networks/new" replace />;
  // }
  return (
    <>
      <TableTitleBar label="Networks">
        <TableMenu label="Add Network">
          <MenuLink label="Create new network" to="/settings/networks/new" />
          <MenuLink label="Add invite code" to="/settings/networks/join" />
        </TableMenu>
      </TableTitleBar>
      <ChatServerTable networks={value.networks} onDelete={() => getServers()} />
    </>
  );
};

export default ChatServerList;
