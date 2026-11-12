// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import HLSEgressService from "../../../../mocks/hlsegress/service";
import { Story } from "../../../../storybook";
import { SettingsStory } from "../Layout.harness";

const PATH = "/settings/video/egress";

class FailingHLSEgressService extends HLSEgressService {
  setConfig(): never {
    throw new Error("listener address is in use");
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
          hlsEgress: new HLSEgressService({ config: { enabled: false, publicServerAddr: "" } }),
        })}
      />
    ),
  },
  {
    // Submitted with a host where the node takes an http(s) URL.
    name: "Invalid",
    component: () => (
      <SettingsStory
        path={PATH}
        submit
        services={() => ({
          hlsEgress: new HLSEgressService({
            config: { enabled: true, publicServerAddr: "cdn.strims.gg" },
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
        services={() => ({ hlsEgress: new FailingHLSEgressService() })}
      />
    ),
  },
];

export default stories;
