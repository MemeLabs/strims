// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate } from "react-router-dom";
import { useTitle } from "react-use";

import { Network } from "../../../apis/strims/network/v1/network";
import {
  MenuCell,
  MenuItem,
  MenuLink,
  Table,
  TableCell,
  TableMenu,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useClient } from "../../../contexts/FrontendApi";
import { certificateRoot } from "../../../lib/certificate";
import PublishNetworkModal from "./PublishNetworkModal";

interface NetworkTableProps {
  networks: Network[];
  onDelete: () => void;
}

const NetworkTable: React.FC<NetworkTableProps> = ({ networks, onDelete }) => {
  const client = useClient();
  const [publishNetwork, setPublishNetwork] = useState<Network>();

  if (!networks) {
    return null;
  }

  const rows = networks.map((network) => {
    const navigate = useNavigate();
    const handleDelete = async () => {
      await client.network.delete({ id: network.id });
      onDelete();
    };

    const handleCreateInvite = () => navigate(`${network.id}/invite`);

    const handlePublish = () => setPublishNetwork(network);

    return (
      <tr key={network.id.toString()}>
        <TableCell>
          <Link to={network.id.toString()}>
            {certificateRoot(network.certificate).subject || "unknown"}
          </Link>
        </TableCell>
        <TableCell>
          {new Date(Number(network.certificate.notAfter) * 1000).toLocaleString()}
        </TableCell>
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

const NetworkList: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const [{ loading, value }, getServers] = useCall("network", "list");

  if (loading) {
    return null;
  }
  return (
    <>
      <TableTitleBar label="Networks">
        <TableMenu label="Add Network">
          <MenuLink label="Create new network" to="/settings/networks/new" />
          <MenuLink label="Add invite code" to="/settings/networks/join" />
        </TableMenu>
      </TableTitleBar>
      <NetworkTable networks={value.networks} onDelete={() => getServers()} />
    </>
  );
};

export default NetworkList;
