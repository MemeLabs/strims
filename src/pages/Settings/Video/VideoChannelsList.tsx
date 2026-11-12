// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, Navigate } from "react-router-dom";
import { useTitle } from "react-use";

import { VideoChannel } from "../../../apis/strims/video/v1/channel";
import { InputError } from "../../../components/Form";
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
  onDeleteError: (error: Error) => void;
}

const VideoChannelTableItem = ({
  channel,
  onDelete,
  onDeleteError,
}: VideoChannelTableItemProps) => {
  const [channelURLRes] = useCall("videoIngress", "getChannelURL", { args: [{ id: channel.id }] });
  const [, deleteChannel] = useLazyCall("videoChannel", "delete", {
    onComplete: onDelete,
    onError: onDeleteError,
  });

  const handleDelete = async () => {
    try {
      await deleteChannel({ id: channel.id });
    } catch {
      // The list shows error.
    }
  };

  const handleCopyKey = () => {
    void navigator.clipboard.writeText(channelURLRes.value.streamKey);
  };

  return (
    <tr>
      <TableCell>
        <Link to={`/settings/video/channels/${channel.id}`}>
          {channel.directoryListingSnippet?.title}
        </Link>
      </TableCell>
      <TableCell truncate>{channelURLRes.value?.url}</TableCell>
      <MenuCell>
        <MenuItem label="Copy Stream Key" onClick={handleCopyKey} />
        <MenuItem label="Delete" onClick={() => void handleDelete()} />
      </MenuCell>
    </tr>
  );
};

const VideoChannelsList = () => {
  const { t } = useTranslation();
  useTitle(t("settings.videoChannel.title"));

  const [channelsRes, listChannels] = useCall("videoChannel", "list");
  // A failed delete's error, until a delete succeeds.
  const [deleteError, setDeleteError] = useState<Error>(null);
  const handleDelete = () => {
    setDeleteError(null);
    void listChannels();
  };

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
        onDelete={handleDelete}
        onDeleteError={setDeleteError}
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
      {deleteError && (
        <div className="thing_form">
          <InputError error={deleteError.message || "Error deleting the channel"} />
        </div>
      )}
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
