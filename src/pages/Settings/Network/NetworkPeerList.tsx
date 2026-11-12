// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useMemo, useState } from "react";
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
  const [error, setError] = useState<string>(null);

  const sortedPeers = useMemo(
    () => (peers ? peers.sort((a, b) => a.alias.localeCompare(b.alias)) : []),
    [peers]
  );

  if (!peers) {
    return null;
  }

  // Runs a row's action, the list reloading after it; a failure shows above
  // the table until the next action.
  const act = async (failure: string, action: () => Promise<unknown>) => {
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(`${failure}: ${(e as Error).message || "unknown error"}`);
      return;
    }
    onChange();
  };

  const handleBan = ({ id, alias, isBanned }: Peer) =>
    act(`Error ${isBanned ? "unbanning" : "banning"} ${alias}`, () =>
      client.network.togglePeerBan({ id, value: !isBanned })
    );
  const handleGrantInvitation = ({ id, alias }: Peer) =>
    act(`Error granting ${alias} an invitation`, () =>
      client.network.grantPeerInvitation({ id, count: 1 })
    );
  const handleResetRenameCooldown = ({ id, alias }: Peer) =>
    act(`Error resetting ${alias}'s rename cooldown`, () =>
      client.network.resetPeerRenameCooldown({ id })
    );
  const handleDelete = ({ id, alias }: Peer) =>
    act(`Error deleting ${alias}`, () => client.network.deletePeer({ id }));

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
          <MenuItem label={peer.isBanned ? "Unban" : "Ban"} onClick={() => void handleBan(peer)} />
          <MenuItem label="Grant invitation" onClick={() => void handleGrantInvitation(peer)} />
          <MenuItem
            label="Reset rename cooldown"
            onClick={() => void handleResetRenameCooldown(peer)}
          />
          <MenuItem label="Delete" onClick={() => void handleDelete(peer)} />
        </MenuCell>
      </tr>
    );
  });

  return (
    <>
      {error && (
        <div className="thing_form">
          <InputError error={error} />
        </div>
      )}
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
