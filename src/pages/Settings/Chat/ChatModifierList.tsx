import React from "react";
import { Link, Navigate, useParams } from "react-router-dom";

import { Modifier } from "../../../apis/strims/chat/v1/chat";
import {
  MenuCell,
  MenuItem,
  MenuLink,
  Table,
  TableMenu,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";

export interface ChatModifierTableProps {
  serverId: bigint;
  modifiers: Modifier[];
  onDelete: () => void;
}

const ChatModifierTable: React.FC<ChatModifierTableProps> = ({ serverId, modifiers, onDelete }) => {
  const [, deleteChatModifier] = useLazyCall("chatServer", "deleteModifier", {
    onComplete: onDelete,
  });

  if (!modifiers) {
    return null;
  }

  const rows = modifiers.map((modifier) => {
    const handleDelete = () => deleteChatModifier({ serverId, id: modifier.id });

    return (
      <tr key={modifier.id.toString()}>
        <td>
          <Link to={`/settings/chat-servers/${serverId}/modifiers/${modifier.id}`}>
            {modifier.name}
          </Link>
        </td>
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

const ChatModifierList: React.FC = () => {
  const { serverId } = useParams<"serverId">();
  const [{ loading, value }, getModifiers] = useCall("chatServer", "listModifiers", {
    args: [{ serverId: BigInt(serverId) }],
  });

  if (loading) {
    return null;
  }
  if (!value?.modifiers.length) {
    return <Navigate to={`/settings/chat-servers/${serverId}/modifiers/new`} />;
  }
  return (
    <>
      <TableTitleBar label="Emote Modifiers" backLink={`/settings/chat-servers/${serverId}`}>
        <TableMenu label="Create">
          <MenuLink
            label="Create Modifier"
            to={`/settings/chat-servers/${serverId}/modifiers/new`}
          />
        </TableMenu>
      </TableTitleBar>
      <ChatModifierTable
        serverId={BigInt(serverId)}
        modifiers={value.modifiers}
        onDelete={() => getModifiers()}
      />
    </>
  );
};

export default ChatModifierList;
