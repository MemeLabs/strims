// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";

import NetworkDirectoryForm, {
  NetworkDirectoryFormData,
  NetworkDirectoryFormProps,
} from "../../../../../pages/Settings/Network/NetworkDirectoryForm";
import { Story } from "../../../../storybook";
import { SettingsStory, Step, submit } from "./harness";

const noop = (): void => undefined;

// Fixture network 1's integrations.
const values: NetworkDirectoryFormData = {
  angelthumpEnable: true,
  twitchEnable: false,
  twitchClientId: "",
  twitchClientSecret: "",
  youtubeEnable: true,
  youtubePublicApiKey: "AIzaSyD-fixture-public-api-key",
  swarmEnable: true,
};

const Form: React.FC<Partial<NetworkDirectoryFormProps> & { steps?: Step[] }> = ({
  steps,
  ...props
}) => (
  <SettingsStory
    steps={steps}
    element={
      <NetworkDirectoryForm
        onSubmit={noop}
        error={null}
        loading={false}
        values={values}
        {...props}
      />
    }
  />
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <Form />,
  },
  {
    name: "Empty",
    component: () => <Form values={undefined} />,
  },
  {
    name: "All enabled",
    component: () => (
      <Form
        values={{
          ...values,
          twitchEnable: true,
          twitchClientId: "x0f1fixtureclientid9k2",
          twitchClientSecret: "fixture-client-secret",
        }}
      />
    ),
  },
  {
    // Twitch and YouTube on without their keys.
    name: "Invalid",
    component: () => (
      <Form
        values={{ ...values, twitchEnable: true, youtubePublicApiKey: "" }}
        steps={[submit()]}
      />
    ),
  },
  {
    name: "Loading",
    component: () => <Form loading={true} />,
  },
  {
    name: "Error",
    component: () => <Form error={new Error({ message: "network 1 not found" })} />,
  },
];

export default stories;
