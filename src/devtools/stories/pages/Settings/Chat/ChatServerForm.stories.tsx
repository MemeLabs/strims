// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Base64 } from "js-base64";
import React from "react";

import ChatServerForm, {
  ChatServerFormData,
  ChatServerFormProps,
} from "../../../../../pages/Settings/Chat/ChatServerForm";
import { Story, fixtures } from "../../../../storybook";
import { Act, ChatSettings, fill, submit } from "./harness";

const noop = (): void => undefined;

const values: ChatServerFormData = {
  name: "Test Network Chat",
  networkKey: Base64.fromUint8Array(fixtures.networkKey(1)),
};

const Form: React.FC<Partial<ChatServerFormProps> & { act?: Act }> = ({ act, ...props }) => (
  <ChatSettings
    path="/settings/chat-servers/1"
    act={act}
    element={
      <ChatServerForm
        onSubmit={noop}
        error={null}
        loading={false}
        submitLabel="Create Server"
        {...props}
      />
    }
  />
);

const stories: Story[] = [
  {
    name: "New",
    component: () => <Form />,
  },
  {
    // An existing server's: its values, and links to its icon, emotes, modifiers and tags.
    name: "Edit",
    component: () => <Form id={BigInt(1)} values={values} submitLabel="Update Server" />,
  },
  {
    // Changed, so it can be saved.
    name: "Edited",
    component: () => (
      <Form
        id={BigInt(1)}
        values={values}
        submitLabel="Update Server"
        act={(root) => fill(root, "Name", "Test Network Lounge")}
      />
    ),
  },
  {
    name: "Invalid",
    component: () => <Form act={submit} />,
  },
  {
    name: "Error",
    component: () => (
      <Form
        id={BigInt(1)}
        values={values}
        submitLabel="Update Server"
        error={new RPCError({ message: "chat server not found" })}
      />
    ),
  },
  {
    name: "Loading",
    component: () => (
      <Form
        id={BigInt(1)}
        values={values}
        submitLabel="Update Server"
        loading
        act={(root) => fill(root, "Name", "Test Network Lounge")}
      />
    ),
  },
];

export default stories;
