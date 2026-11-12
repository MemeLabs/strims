// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { VideoChannel } from "../../../../../apis/strims/video/v1/channel";
import VideoChannelService, { videoChannel } from "../../../../mocks/videochannel/service";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/video/channels/1";

// A channel another node's ingress shares: no network of ours, its logo a URL.
const sharedChannel = (): VideoChannel => {
  const { id, key, token, directoryListingSnippet } = videoChannel(1, {
    channelLogo: { sourceOneof: { url: fixtures.iconUrls[0] } },
  });
  return new VideoChannel({
    id,
    key,
    token,
    directoryListingSnippet,
    owner: { localShare: { certificate: fixtures.network(2).certificate } },
  });
};

class MissingVideoChannelService extends VideoChannelService {
  get(): never {
    throw new Error("channel 1 not found");
  }
}

class FailingVideoChannelService extends VideoChannelService {
  update(): never {
    throw new Error("snippet too large");
  }
}

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Shared",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({
          videoChannel: new VideoChannelService({ channels: [sharedChannel()] }),
        })}
      />
    ),
  },
  {
    // get fails.
    name: "Error",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ videoChannel: new MissingVideoChannelService() })}
      />
    ),
  },
  {
    // Submitted, and update fails.
    name: "Save error",
    component: () => (
      <SettingsStory
        path={PATH}
        submit
        services={() => ({ videoChannel: new FailingVideoChannelService() })}
      />
    ),
  },
];

export default stories;
