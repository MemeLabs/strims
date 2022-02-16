import React from "react";
import { Link, Navigate, useParams } from "react-router-dom";

import { Modifier } from "../../../apis/strims/chat/v1/chat";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import BackLink from "../BackLink";

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
      <div className="thing_list__item" key={modifier.id.toString()}>
        <Link to={`/settings/chat-servers/${serverId}/modifiers/${modifier.id}`}>
          {modifier.name}
        </Link>
        <button className="input input_button" onClick={handleDelete}>
          delete
        </button>
      </div>
    );
  });
  return (
    <div className="thing_list">
      <BackLink
        to={`/settings/chat-servers/${serverId}`}
        title="Server"
        description="Some description of server..."
      />
      {rows}
    </div>
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
      <ChatModifierTable
        serverId={BigInt(serverId)}
        modifiers={value.modifiers}
        onDelete={() => getModifiers()}
      />
      <Link to={`/settings/chat-servers/${serverId}/modifiers/new`}>Create Modifier</Link>
    </>
  );
};

export default ChatModifierList;
