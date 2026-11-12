// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import VideoIngressService from "../../../../mocks/videoingress/service";
import { Catalog, Story } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/video/ingress";

class FailingVideoIngressService extends VideoIngressService {
  setConfig(): never {
    throw new Error("listen tcp 0.0.0.0:1935: address already in use");
  }
}

const stories: Story[] = [
  {
    name: "Enabled",
    component: () => <SettingsStory path={PATH} />,
  },
  {
    name: "Disabled",
    component: () => (
      <SettingsStory
        path={PATH}
        services={() => ({
          videoIngress: new VideoIngressService({
            config: { enabled: false, serverAddr: "127.0.0.1:1935", publicServerAddr: "" },
          }),
        })}
      />
    ),
  },
  {
    // Submitted with a server address without its port and a public
    // address with a scheme (the node puts it in rtmp://<address>/live/…).
    name: "Invalid",
    component: () => (
      <SettingsStory
        path={PATH}
        submit
        services={() => ({
          videoIngress: new VideoIngressService({
            config: {
              enabled: true,
              serverAddr: "127.0.0.1",
              publicServerAddr: "rtmp://ingress.strims.gg",
            },
          }),
        })}
      />
    ),
  },
  {
    // Submitted, and setConfig fails.
    name: "Save error",
    component: () => (
      <SettingsStory
        path={PATH}
        submit
        services={() => ({ videoIngress: new FailingVideoIngressService() })}
      />
    ),
  },
];

export default stories;

// Every Video story mounts the settings route table at its page's path.
export const catalog: Catalog = { covers: ["pages/Settings/Video/index.tsx"] };
