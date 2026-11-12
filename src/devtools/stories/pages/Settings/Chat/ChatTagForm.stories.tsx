// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";

import ChatTagForm, {
  ChatTagFormData,
  ChatTagFormProps,
} from "../../../../../pages/Settings/Chat/ChatTagForm";
import { Story } from "../../../../storybook";
import { Act, ChatSettings, submit } from "./harness";

const nsfw: ChatTagFormData = { name: "nsfw", color: "#e53935", sensitive: true };
const loud: ChatTagFormData = { name: "loud", color: "#fdd835", sensitive: false };

const noop = (): void => undefined;

const Form: React.FC<Partial<ChatTagFormProps> & { act?: Act }> = ({ act, ...props }) => (
  <ChatSettings
    path="/settings/chat-servers/1/tags/new"
    act={act}
    element={
      <ChatTagForm
        onSubmit={noop}
        error={null}
        loading={false}
        submitLabel={props.values ? "Update Tag" : "Create Tag"}
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
    name: "Edit",
    component: () => <Form values={loud} />,
  },
  {
    // A tag that hides what it marks till asked.
    name: "Sensitive",
    component: () => <Form values={nsfw} />,
  },
  {
    name: "Invalid",
    component: () => <Form act={submit} />,
  },
  {
    name: "Error",
    component: () => <Form values={nsfw} error={new RPCError({ message: "tag name in use" })} />,
  },
  {
    name: "Loading",
    component: () => <Form values={nsfw} loading />,
  },
];

export default stories;
