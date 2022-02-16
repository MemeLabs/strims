import React, { useEffect, useState } from "react";
import { Link, Navigate, useParams } from "react-router-dom";

import { Emote, EmoteImage } from "../../../apis/strims/chat/v1/chat";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import BackLink from "../BackLink";
import { fileTypeToMimeType, scaleToDOMScale } from "./utils";

interface ImageProps {
  src: EmoteImage;
}

const Image: React.FC<ImageProps> = ({ src }) => {
  const [url] = useState(() =>
    URL.createObjectURL(new Blob([src.data], { type: fileTypeToMimeType(src.fileType) }))
  );
  useEffect(() => () => URL.revokeObjectURL(url));

  return <img srcSet={`${url} ${scaleToDOMScale(src.scale)}`} />;
};

export interface ChatEmoteTableProps {
  serverId: bigint;
  emotes: Emote[];
  onDelete: () => void;
}

const ChatEmoteTable: React.FC<ChatEmoteTableProps> = ({ serverId, emotes, onDelete }) => {
  const [, deleteChatEmote] = useLazyCall("chatServer", "deleteEmote", { onComplete: onDelete });

  if (!emotes) {
    return null;
  }

  const rows = emotes.map((emote) => {
    const handleDelete = () => deleteChatEmote({ serverId, id: emote.id });

    return (
      <div className="thing_list__item" key={emote.id.toString()}>
        <Image src={emote.images[0]} />
        <Link to={`/settings/chat-servers/${serverId}/emotes/${emote.id}`}>{emote.name}</Link>
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

const ChatEmoteList: React.FC = () => {
  const { serverId } = useParams<"serverId">();
  const [{ loading, value }, getEmotes] = useCall("chatServer", "listEmotes", {
    args: [{ serverId: BigInt(serverId) }],
  });

  if (loading) {
    return null;
  }
  if (!value?.emotes.length) {
    return <Navigate to={`/settings/chat-servers/${serverId}/emotes/new`} />;
  }
  return (
    <>
      <ChatEmoteTable
        serverId={BigInt(serverId)}
        emotes={value.emotes}
        onDelete={() => getEmotes()}
      />
      <Link to={`/settings/chat-servers/${serverId}/emotes/new`}>Create Emote</Link>
    </>
  );
};

export default ChatEmoteList;
