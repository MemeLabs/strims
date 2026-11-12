// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";

import NetworkForm, { NetworkFormProps } from "../../../../../pages/Settings/Network/NetworkForm";
import { Story } from "../../../../storybook";
import { SettingsStory, Step, fill, submit } from "./harness";

const noop = (): void => undefined;

const Form: React.FC<Partial<NetworkFormProps> & { steps?: Step[] }> = ({ steps, ...props }) => (
  <SettingsStory
    steps={steps}
    element={
      <NetworkForm
        onSubmit={noop}
        error={null}
        loading={false}
        networkId={BigInt(1)}
        showDirectoryFormLink={true}
        values={{ alias: "test_user" }}
        {...props}
      />
    }
  />
);

const stories: Story[] = [
  {
    // An owner's network: the links to its directory, icon, peers and reservations.
    name: "Default",
    component: () => <Form />,
  },
  {
    name: "Member",
    component: () => <Form showDirectoryFormLink={false} />,
  },
  {
    name: "Empty",
    component: () => <Form values={undefined} showDirectoryFormLink={false} />,
  },
  {
    name: "Loading",
    component: () => <Form loading={true} />,
  },
  {
    name: "Error",
    component: () => <Form error={new Error({ message: "alias test_user is reserved" })} />,
  },
  {
    name: "Invalid",
    component: () => <Form steps={[fill("alias", "not a valid name!"), submit()]} />,
  },
  {
    name: "Long alias",
    component: () => <Form values={{ alias: "a_twenty_char_alias_" }} />,
  },
];

export default stories;
