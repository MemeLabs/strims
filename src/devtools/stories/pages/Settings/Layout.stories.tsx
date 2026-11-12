// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import * as hlsv1 from "../../../../apis/strims/video/v1/hls_egress";
import * as ingressv1 from "../../../../apis/strims/video/v1/ingress";
import HLSEgressService from "../../../mocks/hlsegress/service";
import VideoIngressService from "../../../mocks/videoingress/service";
import { Story } from "../../../storybook";
import { SettingsStory, later } from "./Layout.harness";

// Both isSupported calls answer after 3 s: the nav hides its links until then.
class LateVideoIngressService extends VideoIngressService {
  isSupported() {
    return later(3000, new ingressv1.VideoIngressIsSupportedResponse({ supported: true }));
  }
}

class LateHLSEgressService extends HLSEgressService {
  isSupported() {
    return later(3000, new hlsv1.HLSEgressIsSupportedResponse({ supported: true }));
  }
}

const stories: Story[] = [
  {
    // The node serves RTMP ingress and HLS egress: the nav links both.
    name: "Default",
    component: () => <SettingsStory nav path="/settings/video/ingress" />,
  },
  {
    name: "Unsupported",
    component: () => (
      <SettingsStory
        nav
        path="/settings/autoseed/config"
        services={() => ({
          videoIngress: new VideoIngressService({ supported: false }),
          hlsEgress: new HLSEgressService({ supported: false }),
        })}
      />
    ),
  },
  {
    name: "Loading",
    component: () => (
      <SettingsStory
        nav
        path="/settings/debug"
        services={() => ({
          videoIngress: new LateVideoIngressService(),
          hlsEgress: new LateHLSEgressService(),
        })}
      />
    ),
  },
];

export default stories;
