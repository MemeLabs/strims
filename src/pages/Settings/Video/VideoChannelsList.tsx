import { Base64 } from "js-base64";
import React from "react";

import { VideoChannel } from "../../../apis/strims/video/v1/channel";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import jsonutil from "../../../lib/jsonutil";
import VideoIngressChannelForm, { VideoIngressChannelFormData } from "./VideoChannelForm";

interface VideoChannelTableItemProps {
  channel: VideoChannel;
  onDelete: () => void;
}

const VideoChannelTableItem = ({ channel, onDelete }: VideoChannelTableItemProps) => {
  const [channelURLRes] = useCall("videoIngress", "getChannelURL", { args: [{ id: channel.id }] });

  return (
    <div className="thing_list__item">
      <div>
        <div>{channel.directoryListingSnippet?.title}</div>
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
  const [, createChannel] = useLazyCall("videoChannel", "create");
  const [, deleteChannel] = useLazyCall("videoChannel", "delete", {
    onComplete: listChannels,
  });

  const handleSubmit = React.useCallback(async (data: VideoIngressChannelFormData) => {
    await createChannel({
      directoryListingSnippet: {
        title: data.title,
        description: data.description,
        tags: data.tags.map(({ value }) => value),
      },
      networkKey: Base64.toUint8Array(data.networkKey.value),
    });
    void listChannels();
  }, []);

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
      <VideoIngressChannelForm onSubmit={handleSubmit} />
      <div className="thing_list">
        <div>Active Channels ({rows?.length || 0})</div>
        {rows}
      </div>
    </>
  );
};

export default VideoChannelsList;
