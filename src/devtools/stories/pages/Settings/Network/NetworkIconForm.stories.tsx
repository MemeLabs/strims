// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React, { useEffect, useState } from "react";

import { ImageValue } from "../../../../../components/Form";
import { toFormImageValue } from "../../../../../lib/image";
import NetworkIconForm, {
  NetworkIconFormProps,
} from "../../../../../pages/Settings/Network/NetworkIconForm";
import { Story, fixtures } from "../../../../storybook";
import { SettingsStory, Step, pngImage, submit } from "./harness";

const noop = (): void => undefined;

type FormProps = Partial<NetworkIconFormProps> & { steps?: Step[] };

const Form: React.FC<FormProps> = ({ steps, ...props }) => (
  <SettingsStory
    steps={steps}
    element={
      <NetworkIconForm
        onSubmit={noop}
        error={null}
        loading={false}
        submitLabel="Update Icon"
        {...props}
      />
    }
  />
);

// The form once its icon (a fixture emote) has loaded: the input reads its
// value at mount.
const WithIcon: React.FC<FormProps> = (props) => {
  const [image, setImage] = useState<ImageValue>();
  useEffect(() => {
    void pngImage(fixtures.iconUrls[0]).then((i) => setImage(toFormImageValue(i)));
  }, []);
  return image ? <Form values={{ image }} {...props} /> : null;
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => <WithIcon />,
  },
  {
    name: "Empty",
    component: () => <Form submitLabel="Create Icon" />,
  },
  {
    name: "Invalid",
    component: () => <Form steps={[submit()]} />,
  },
  {
    name: "Loading",
    component: () => <WithIcon loading={true} />,
  },
  {
    name: "Error",
    component: () => <WithIcon error={new Error({ message: "image too large" })} />,
  },
];

export default stories;
