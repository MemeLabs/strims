// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Link, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { Peer } from "../../../apis/strims/network/v1/network";
import { InputError } from "../../../components/Form";
import {
  MenuCell,
  MenuItem,
  Table,
  TableCell,
  TableHeader,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useClient } from "../../../contexts/FrontendApi";

interface NetworkTableProps {
  peers: Peer[];
  onChange: () => void;
}

const PeerTable: React.FC<NetworkTableProps> = ({ peers, onChange }) => {
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
        <TableCell truncate>
          <Link to={peer.id.toString()}>{peer.alias}</Link>
        </TableCell>
        <TableCell showFor="md">
          {new Date(Number(peer.createdAt) * 1000).toLocaleString()}
        </TableCell>
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

  return (
    <>
      <Table>
        <thead>
          <tr>
            <th>Name</th>
            <TableHeader showFor="md">Created at</TableHeader>
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
  const [{ loading, value, error }, listPeers] = useCall("network", "listPeers", {
    args: [{ networkId: BigInt(networkId) }],
  });

  if (loading) {
    return null;
  }
  return (
    <>
      <TableTitleBar label="Network peers" backLink={`/settings/networks/${networkId}`} />
      {error ? (
        <div className="thing_form">
          <InputError error={error.message || "Error loading peers"} />
        </div>
      ) : (
        <PeerTable peers={value.peers} onChange={listPeers} />
      )}
    </>
  );
};

export default NetworkPeerList;
