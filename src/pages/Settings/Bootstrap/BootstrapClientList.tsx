// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Link, Navigate } from "react-router-dom";
import { useTitle } from "react-use";

import { BootstrapClient } from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import {
  MenuCell,
  MenuItem,
  MenuLink,
  Table,
  TableCell,
  TableMenu,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";

interface BootstrapTableItemProps {
  client: BootstrapClient;
  onDelete: () => void;
}

const BootstrapTableItem = ({ client, onDelete }: BootstrapTableItemProps) => {
  const [, deleteClient] = useLazyCall("bootstrap", "deleteClient", {
    onComplete: onDelete,
  });

  const handleDelete = useCallback(() => deleteClient({ id: client.id }), [client]);

  let url: string;
  switch (client.clientOptions.case) {
    case BootstrapClient.ClientOptionsCase.WEBSOCKET_OPTIONS:
      url = client.clientOptions.websocketOptions.url;
      break;
    default:
      return null;
  }

  return (
    <tr>
      <TableCell truncate>
        <Link to={`/settings/bootstrap/clients/${client.id}`}>{url}</Link>
      </TableCell>
      <MenuCell>
        <MenuItem label="Delete" onClick={handleDelete} />
      </MenuCell>
    </tr>
  );
};

const BootstrapsList = () => {
  const { t } = useTranslation();
  useTitle(t("settings.bootstrap.title"));

  const [clientsRes, listClients] = useCall("bootstrap", "listClients");

  if (clientsRes.loading) {
    return null;
  }
  if (!clientsRes.value?.bootstrapClients.length) {
    return <Navigate to="/settings/bootstrap/clients/new" />;
  }

  const rows = clientsRes.value?.bootstrapClients?.map((client) => {
    return <BootstrapTableItem key={client.id.toString()} client={client} onDelete={listClients} />;
  });

  return (
    <>
      <TableTitleBar label="Boostraps" backLink="/settings/bootstrap">
        <TableMenu label="Create">
          <MenuLink label="Create Client" to="/settings/bootstrap/clients/new" />
        </TableMenu>
      </TableTitleBar>
      <Table>
        <thead>
          <tr>
            <th>URL</th>
            <th></th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </Table>
    </>
  );
};

export default BootstrapsList;
