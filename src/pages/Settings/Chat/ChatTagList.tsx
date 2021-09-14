import React from "react";
import { Link, Redirect, useParams } from "react-router-dom";

import { Tag } from "../../../apis/strims/chat/v1/chat";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import BackLink from "./BackLink";

export interface ChatTagTableProps {
  serverId: bigint;
  tags: Tag[];
  onDelete: () => void;
}

const ChatTagTable: React.FC<ChatTagTableProps> = ({ serverId, tags, onDelete }) => {
  const [, deleteChatTag] = useLazyCall("chat", "deleteTag", { onComplete: onDelete });

  if (!tags) {
    return null;
  }

  const rows = tags.map((modifier) => {
    const handleDelete = () => deleteChatTag({ serverId, id: modifier.id });

    return (
      <div className="thing_list__item" key={modifier.id.toString()}>
        <Link to={`/settings/chat-servers/${serverId}/tags/${modifier.id}`}>{modifier.name}</Link>
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

const ChatTagList: React.FC = () => {
  const { serverId } = useParams<{ serverId: string }>();
  const [{ loading, value }, getTags] = useCall("chat", "listTags", {
    args: [{ serverId: BigInt(serverId) }],
  });

  if (loading) {
    return null;
  }
  if (!value?.tags.length) {
    return <Redirect to={`/settings/chat-servers/${serverId}/tags/new`} />;
  }
  return (
    <>
      <ChatTagTable serverId={BigInt(serverId)} tags={value.tags} onDelete={() => getTags()} />
      <Link to={`/settings/chat-servers/${serverId}/tags/new`}>Create Tag</Link>
    </>
  );
};

export default ChatTagList;
