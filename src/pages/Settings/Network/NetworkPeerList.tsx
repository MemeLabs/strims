// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { Network, Peer } from "../../../apis/strims/network/v1/network";
import {
  MenuCell,
  MenuItem,
  Table,
  TableCell,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useClient } from "../../../contexts/FrontendApi";
import PublishNetworkModal from "./PublishNetworkModal";

interface NetworkTableProps {
  peers: Peer[];
  onChange: () => void;
}

const PeerTable: React.FC<NetworkTableProps> = ({ peers, onChange }) => {
  const [publishNetwork, setPublishNetwork] = useState<Network>();
  const client = useClient();

  const sortedPeers = useMemo(
    () => (peers ? peers.sort((a, b) => a.alias.localeCompare(b.alias)) : []),
    [peers]
  );

  if (!peers) {
    return null;
  }

  const handleGrantInvitation = async (id: bigint) => {
    await client.network.grantPeerInvitation({ id, count: 1 });
    onChange();
  };
  const handleBan = async (id: bigint, value: boolean) => {
    await client.network.togglePeerBan({ id, value });
    onChange();
  };
  const handleResetRenameCooldown = async (id: bigint) => {
    await client.network.resetPeerRenameCooldown({ id });
    onChange();
  };
  const handleDelete = async (id: bigint) => {
    await client.network.deletePeer({ id });
    onChange();
  };

  const rows = sortedPeers.map((peer) => {
    return (
      <tr key={peer.id.toString()}>
        <TableCell>
          <Link to={peer.id.toString()}>{peer.alias}</Link>
        </TableCell>
        <TableCell>{new Date(Number(peer.createdAt) * 1000).toLocaleString()}</TableCell>
        <TableCell>{peer.isBanned ? "true" : "false"}</TableCell>
        <MenuCell>
          <MenuItem
            label={peer.isBanned ? "Unban" : "Ban"}
            onClick={() => handleBan(peer.id, !peer.isBanned)}
          />
          <MenuItem label="Grant invitation" onClick={() => handleGrantInvitation(peer.id)} />
          <MenuItem
            label="Reset rename cooldown"
            onClick={() => handleResetRenameCooldown(peer.id)}
          />
          <MenuItem label="Delete" onClick={() => handleDelete(peer.id)} />
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
            <th>Created at</th>
            <th>Banned</th>
            <th />
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </Table>
    </>
  );
};

const NetworkPeerList: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const { networkId } = useParams<"networkId">();
  const [{ loading, value }, listPeers] = useCall("network", "listPeers", {
    args: [{ networkId: BigInt(networkId) }],
  });

  if (loading) {
    return null;
  }
  return (
    <>
      <TableTitleBar label="Network peers" backLink={`/settings/networks/${networkId}`} />
      <PeerTable peers={value.peers} onChange={listPeers} />
    </>
  );
};

export default NetworkPeerList;
