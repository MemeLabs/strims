// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Link, Navigate } from "react-router-dom";
import { useTitle } from "react-use";

import { VideoChannel } from "../../../apis/strims/video/v1/channel";
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

interface VideoChannelTableItemProps {
  channel: VideoChannel;
  onDelete: () => void;
}

const VideoChannelTableItem = ({ channel, onDelete }: VideoChannelTableItemProps) => {
  const [channelURLRes] = useCall("videoIngress", "getChannelURL", { args: [{ id: channel.id }] });
  const [, deleteChannel] = useLazyCall("videoChannel", "delete", {
    onComplete: onDelete,
  });

  const handleDelete = useCallback(() => deleteChannel({ id: channel.id }), [channel]);

  const handleCopyKey = () => {
    void navigator.clipboard.writeText(channelURLRes.value.streamKey);
    console.log("copied stream key to clipboard");
  };

  return (
    <tr>
      <td>
        <Link to={`/settings/video/channels/${channel.id}`}>
          {channel.directoryListingSnippet?.title}
        </Link>
      </td>
      <TableCell truncate>{channelURLRes.value?.url}</TableCell>
      <MenuCell>
        <MenuItem label="Copy Stream Key" onClick={handleCopyKey} />
        <MenuItem label="Delete" onClick={handleDelete} />
      </MenuCell>
    </tr>
  );
};

const VideoChannelsList = () => {
  const { t } = useTranslation();
  useTitle(t("settings.videoChannel.title"));

  const [channelsRes, listChannels] = useCall("videoChannel", "list");

  if (channelsRes.loading) {
    return null;
  }
  if (!channelsRes.value?.channels.length) {
    return <Navigate to="/settings/video/channels/new" />;
  }

  const rows = channelsRes.value?.channels?.map((channel) => {
    return (
      <VideoChannelTableItem
        key={channel.id.toString()}
        channel={channel}
        onDelete={listChannels}
      />
    );
  });

  return (
    <>
      <TableTitleBar label="Channels" backLink="/settings/video/ingress">
        <TableMenu label="Create">
          <MenuLink label="Create channel" to="/settings/video/channels/new" />
        </TableMenu>
      </TableTitleBar>
      <Table>
        <thead>
          <tr>
            <th>Title</th>
            <th>URL</th>
            <th></th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </Table>
    </>
  );
};

export default VideoChannelsList;
