import React from "react";
import { MdChevronLeft } from "react-icons/md";
import { Link, Navigate } from "react-router-dom";

import { VideoChannel } from "../../../apis/strims/video/v1/channel";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import jsonutil from "../../../lib/jsonutil";

interface VideoChannelTableItemProps {
  channel: VideoChannel;
  onDelete: () => void;
}

const VideoChannelTableItem = ({ channel, onDelete }: VideoChannelTableItemProps) => {
  const [channelURLRes] = useCall("videoIngress", "getChannelURL", { args: [{ id: channel.id }] });

  return (
    <div className="thing_list__item">
      <div>
        <Link to={`/settings/video/channels/${channel.id}`}>
          {channel.directoryListingSnippet?.title}
        </Link>
        <div>{channel.directoryListingSnippet?.description}</div>
        <div>{channelURLRes.value?.url}</div>
        <div>
          {channel.directoryListingSnippet?.tags.map((tag, i) => (
            <span key={i}>{tag}</span>
          ))}
        </div>
      </div>
      <button className="input input_button" onClick={onDelete}>
        delete
      </button>
      <pre>{jsonutil.stringify(channel)}</pre>
    </div>
  );
};

const VideoChannelsList = () => {
  const [channelsRes, listChannels] = useCall("videoChannel", "list");
  const [, deleteChannel] = useLazyCall("videoChannel", "delete", {
    onComplete: listChannels,
  });

  if (channelsRes.loading) {
    return null;
  }
  if (!channelsRes.value?.channels.length) {
    return <Navigate to="/settings/videos/channels/new" />;
  }

  const rows = channelsRes.value?.channels?.map((channel) => {
    return (
      <VideoChannelTableItem
        key={channel.id.toString()}
        channel={channel}
        onDelete={() => deleteChannel({ id: channel.id })}
      />
    );
  });

  return (
    <>
      <Link to="/settings/video/channels/new">Create channel</Link>
      <div className="thing_list">
        <Link className="input_label input_label--button" to="/settings/video">
          <MdChevronLeft size="28" />
          <div className="input_label__body">
            <div>Ingress</div>
            <div>Some description of ingress...</div>
          </div>
        </Link>
        {rows}
      </div>
    </>
  );
};

export default VideoChannelsList;
