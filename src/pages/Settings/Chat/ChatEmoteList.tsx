// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./ChatEmoteList.scss";

import { escapeRegExp } from "lodash";
import React, {
  ChangeEvent,
  MouseEvent,
  MutableRefObject,
  ReactNode,
  useCallback,
  useRef,
  useState,
} from "react";
import { useTranslation } from "react-i18next";
import { BsDot } from "react-icons/bs";
import { Link, Navigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { Emote, ListEmotesRequest } from "../../../apis/strims/chat/v1/chat";
import { Button } from "../../../components/Form";
import Label from "../../../components/Settings/Label";
import Search from "../../../components/Settings/Search";
import {
  CheckboxCell,
  CheckboxHeader,
  MenuCell,
  MenuItem,
  MenuLink,
  Table,
  TableCell,
  TableMenu,
  TableState,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useClient, useLazyCall } from "../../../contexts/FrontendApi";

export interface ChatEmoteTableProps {
  serverId: bigint;
  emotes: Emote[];
  onDelete: () => void;
  search: string;
  tableRef: MutableRefObject<TableState>;
}

const ChatEmoteTable: React.FC<ChatEmoteTableProps> = ({
  serverId,
  emotes,
  onDelete,
  search,
  tableRef,
}) => {
  const [, deleteChatEmote] = useLazyCall("chatServer", "deleteEmote", { onComplete: onDelete });

  if (!emotes) {
    return null;
  }

  const pattern = new RegExp(escapeRegExp(search), "i");
  const rows: ReactNode[] = [];
  for (const emote of emotes) {
    if (!emote.name.match(pattern) && !emote.labels.some((l) => l.match(pattern))) {
      continue;
    }

    const handleDelete = () => deleteChatEmote({ serverId, id: emote.id });

    rows.push(
      <tr key={emote.id.toString()}>
        <CheckboxCell name="id" id={emote.id} />
        <TableCell className="chat_emote_list__enable_cell">
          {emote.enable && <BsDot className="chat_emote_list__enable_indicator" />}
        </TableCell>
        <TableCell>
          <Link to={`/settings/chat-servers/${serverId}/emotes/${emote.id}`}>{emote.name}</Link>
        </TableCell>
        <TableCell>
          {emote.labels.map((l) => (
            <Label key={l}>{l}</Label>
          ))}
        </TableCell>
        <MenuCell>
          <MenuItem label="Delete" onClick={handleDelete} />
        </MenuCell>
      </tr>
    );
  }
  return (
    <Table ref={tableRef}>
      <thead>
        <tr>
          <CheckboxHeader name="id" />
          <th></th>
          <th>Title</th>
          <th></th>
          <th></th>
        </tr>
      </thead>
      <tbody>{rows}</tbody>
    </Table>
  );
};

const ChatEmoteList: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.chat.title"));

  const { serverId } = useParams<"serverId">();
  const [{ loading, value }, getEmotes] = useCall("chatServer", "listEmotes", {
    args: [
      {
        serverId: BigInt(serverId),
        parts: [ListEmotesRequest.Part.PART_META],
      },
    ],
  });

  const [search, setSearch] = useState("");
  const handleSearchChange = useCallback((e: ChangeEvent<HTMLInputElement>) => {
    setSearch(e.currentTarget.value);
  }, []);

  const table = useRef<TableState>(null);
  const client = useClient();
  const updateEmotesEnabled = async (value: boolean): Promise<void> => {
    await client.chatServer.updateEmotes({
      serverId: BigInt(serverId),
      ids: [...new Set(table.current.values.get("id"))],
      enable: { value },
    });
    void getEmotes();
  };

  const handleEnableClick = useCallback(() => void updateEmotesEnabled(true), []);
  const handleDisableClick = useCallback(() => void updateEmotesEnabled(false), []);

  if (loading) {
    return null;
  }
  if (!value?.emotes.length) {
    return <Navigate to={`/settings/chat-servers/${serverId}/emotes/new`} />;
  }
  return (
    <>
      <TableTitleBar label="Emotes" backLink={`/settings/chat-servers/${serverId}`}>
        <Search onChange={handleSearchChange} placeholder="Search" />
        <Button borderless onClick={handleEnableClick}>
          Enable Selected
        </Button>
        <Button borderless onClick={handleDisableClick}>
          Disable Selected
        </Button>
        <TableMenu label="Create">
          <MenuLink label="Create Emote" to={`/settings/chat-servers/${serverId}/emotes/new`} />
        </TableMenu>
      </TableTitleBar>
      <ChatEmoteTable
        serverId={BigInt(serverId)}
        emotes={value.emotes}
        onDelete={() => getEmotes()}
        search={search}
        tableRef={table}
      />
    </>
  );
};

export default ChatEmoteList;
