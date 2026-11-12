// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";

import { toFormImageValue } from "../../../../../lib/image";
import ChatServerIconForm, {
  ChatServerIconFormProps,
} from "../../../../../pages/Settings/Chat/ChatServerIconForm";
import { serverIcon } from "../../../../mocks/chatserver/fixtures";
import { Story } from "../../../../storybook";
import { Act, ChatSettings, submit, useLoaded } from "./harness";

const noop = (): void => undefined;

interface FormProps extends Partial<ChatServerIconFormProps> {
  /** Starts with server 1's fixture icon. */
  icon?: boolean;
  act?: Act;
}

const Form: React.FC<FormProps> = ({ icon, act, ...props }) => {
  const image = useLoaded(async () =>
    icon ? toFormImageValue((await serverIcon(BigInt(1))).image) : null
  );
  if (image === undefined) {
    return null;
  }
  return (
    <ChatSettings
      path="/settings/chat-servers/1/icon"
      act={act}
      element={
        <ChatServerIconForm
          onSubmit={noop}
          error={null}
          loading={false}
          values={image ? { image } : undefined}
          submitLabel="Update Server Icon"
          {...props}
        />
      }
    />
  );
};

const stories: Story[] = [
  {
    name: "Empty",
    component: () => <Form />,
  },
  {
    name: "Image",
    component: () => <Form icon />,
  },
  {
    name: "Invalid",
    component: () => <Form act={submit} />,
  },
  {
    name: "Error",
    component: () => <Form icon error={new RPCError({ message: "image too large" })} />,
  },
  {
    name: "Loading",
    component: () => <Form icon loading />,
  },
];

export default stories;
