// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import VideoChannelService, { videoChannel } from "../../../../mocks/videochannel/service";
import VideoIngressService from "../../../../mocks/videoingress/service";
import { Story } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/video/channels";

const stories: Story[] = [
  {
    name: "Default",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Long text",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({
          videoIngress: new VideoIngressService({
            config: {
              enabled: true,
              serverAddr: "0.0.0.0:1935",
              publicServerAddr:
                "rtmp-ingress.eu-central.example-broadcasting-network.strims.gg:19350",
            },
          }),
          videoChannel: new VideoChannelService({
            channels: [
              videoChannel(1, {
                title:
                  "A channel whose title goes on and on, far past what the table's first column can show in one line",
              }),
              ...[2, 3, 4, 5, 6, 7, 8].map((i) => videoChannel(i)),
            ],
          }),
        })}
      />
    ),
  },
  {
    // No channels: the list sends its visitor to the create form.
    name: "Empty",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({ videoChannel: new VideoChannelService({ channels: [] }) })}
      />
    ),
  },
];

export default stories;
