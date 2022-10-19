// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { Link, Navigate } from "react-router-dom";
import { useTitle } from "react-use";

import { Server } from "../../../apis/strims/chat/v1/chat";
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

interface ChatServerTableProps {
  servers: Server[];
  onDelete: () => void;
}

const ChatServerTable: React.FC<ChatServerTableProps> = ({ servers, onDelete }) => {
  const [{ error }, deleteChatServer] = useLazyCall("chatServer", "deleteServer", {
    onComplete: onDelete,
  });

  if (!servers) {
    return null;
  }

  const rows = servers.map((server) => {
    const handleDelete = () => deleteChatServer({ id: server.id });

    return (
      <tr key={server.id.toString()}>
        <TableCell>
          <Link to={`/settings/chat-servers/${server.id}`}>{server.room.name || "no title"}</Link>
        </TableCell>
        <MenuCell>
          <MenuItem label="Delete" onClick={handleDelete} />
        </MenuCell>
      </tr>
    );
  });
  return (
    <Table>
      <thead>
        <tr>
          <th>Title</th>
          <th></th>
        </tr>
      </thead>
      <tbody>{rows}</tbody>
    </Table>
  );
};

const ChatServerList: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const [{ loading, value }, getServers] = useCall("chatServer", "listServers");

  if (loading) {
    return null;
  }
  if (!value?.servers.length) {
    return <Navigate to="/settings/chat-servers/new" />;
  }
  return (
    <>
      <TableTitleBar label="Chat Servers">
        <TableMenu label="Create">
          <MenuLink label="Create Server" to="/settings/chat-servers/new" />
        </TableMenu>
      </TableTitleBar>
      <ChatServerTable servers={value.servers} onDelete={() => getServers()} />
    </>
  );
};

export default ChatServerList;
