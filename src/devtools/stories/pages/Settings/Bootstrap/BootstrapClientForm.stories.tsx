// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";

import BootstrapForm, {
  BootstrapFormProps,
} from "../../../../../pages/Settings/Bootstrap/BootstrapClientForm";
import { Story } from "../../../../storybook";
import { SettingsStory, Step, fill, submit } from "../Network/harness";

const noop = (): void => undefined;

const Form: React.FC<Partial<BootstrapFormProps> & { steps?: Step[] }> = ({ steps, ...props }) => (
  <SettingsStory
    steps={steps}
    element={
      <BootstrapForm
        onSubmit={noop}
        error={null}
        loading={false}
        submitLabel="Update Bootstrap"
        values={{ url: "wss://strims.gg/_bootstrap", insecureSkipVerifyTls: false }}
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
    component: () => <Form values={undefined} submitLabel="Create Bootstrap" />,
  },
  {
    name: "Insecure",
    component: () => (
      <Form values={{ url: "wss://192.168.1.20:8082/_bootstrap", insecureSkipVerifyTls: true }} />
    ),
  },
  {
    name: "Long URL",
    component: () => (
      <Form
        values={{
          url: "wss://a-very-long-bootstrap-host-name.eu-west-1.relay.example.net:8082/_bootstrap/with/a/long/path?token=0123456789abcdef0123456789abcdef",
          insecureSkipVerifyTls: false,
        }}
      />
    ),
  },
  {
    name: "Required",
    component: () => <Form values={undefined} steps={[submit()]} />,
  },
  {
    name: "Invalid",
    component: () => <Form steps={[fill("url", "not a url"), submit()]} />,
  },
  {
    name: "Loading",
    component: () => <Form loading={true} />,
  },
  {
    name: "Error",
    component: () => <Form error={new Error({ message: "bootstrap client 9 not found" })} />,
  },
];

export default stories;
