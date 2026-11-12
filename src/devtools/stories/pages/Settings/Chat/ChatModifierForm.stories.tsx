// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error as RPCError } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";

import { toFormImageValue } from "../../../../../lib/image";
import ChatModifierForm, {
  ChatModifierFormData,
  ChatModifierFormProps,
} from "../../../../../pages/Settings/Chat/ChatModifierForm";
import { emoteFixture, pngImage } from "../../../../mocks/chatserver/fixtures";
import { Story } from "../../../../storybook";
import { Act, ChatSettings, submit, useLoaded } from "./harness";

const mirror: ChatModifierFormData = {
  name: "mirror",
  priority: 0,
  internal: false,
  extraWrapCount: 0,
  procChance: 0,
  scss: ".mirror {\n  transform: scaleX(-1);\n}\n",
  assetCount: 0,
};

// A modifier whose style sheet draws an image of its own.
const banned = async (): Promise<ChatModifierFormData> => ({
  name: "banned",
  priority: 1,
  internal: false,
  extraWrapCount: 0,
  procChance: 0,
  scss: '.banned:after {\n  content: "";\n  background-image: url("RedCard");\n}\n',
  assetCount: 1,
  asset_0_name: "RedCard",
  asset_0_image: toFormImageValue(await pngImage(emoteFixture("RedCard").url)),
});

// One the server adds to messages at random.
const virus: ChatModifierFormData = {
  name: "virus",
  priority: 0,
  internal: true,
  extraWrapCount: 0,
  procChance: 0.0001,
  scss: "",
  assetCount: 0,
};

const noop = (): void => undefined;

interface FormProps extends Partial<ChatModifierFormProps> {
  load?: () => Promise<ChatModifierFormData>;
  act?: Act;
}

const Form: React.FC<FormProps> = ({ load, act, values: valuesProp, ...props }) => {
  const values = useLoaded(async () => (load ? load() : (valuesProp ?? null)));
  if (values === undefined) {
    return null;
  }
  return (
    <ChatSettings
      path="/settings/chat-servers/1/modifiers/new"
      act={act}
      element={
        <ChatModifierForm
          onSubmit={noop}
          error={null}
          loading={false}
          values={values ?? undefined}
          submitLabel={values ? "Update Modifier" : "Create Modifier"}
          {...props}
        />
      }
    />
  );
};

const stories: Story[] = [
  {
    name: "New",
    component: () => <Form />,
  },
  {
    name: "Edit",
    component: () => <Form values={mirror} />,
  },
  {
    // A style sheet with an image asset.
    name: "Asset",
    component: () => <Form load={banned} />,
  },
  {
    // Internal: its proc chance enabled.
    name: "Internal",
    component: () => <Form values={virus} />,
  },
  {
    name: "Invalid",
    component: () => <Form act={submit} />,
  },
  {
    name: "Error",
    component: () => (
      <Form values={mirror} error={new RPCError({ message: "modifier name in use" })} />
    ),
  },
  {
    name: "Loading",
    component: () => <Form values={mirror} loading />,
  },
];

export default stories;
