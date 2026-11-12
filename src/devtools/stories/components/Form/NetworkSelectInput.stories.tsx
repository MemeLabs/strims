// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { ReactNode } from "react";

import { NetworkSelectInput } from "../../../../components/Form";
import NetworkService from "../../../mocks/network/service";
import { MockApi, Story, fixtures } from "../../../storybook";
import { FormStory } from "./harness";

// The input's values: networks' root keys, base64.
const networkValue = (i: number) => Base64.fromUint8Array(fixtures.networkKey(i));

interface Values {
  network: string;
  networks: string[];
}

const required = { required: { value: true, message: "Pick a network" } };

const Networks: React.FC<{ count?: number; children: ReactNode }> = ({ count = 5, children }) => (
  <MockApi services={() => ({ network: new NetworkService({ count }) })}>{children}</MockApi>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Networks>
        <FormStory<Values>>
          {(control) => <NetworkSelectInput control={control} name="network" label="Network" />}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "With value",
    component: () => (
      <Networks>
        <FormStory<Values> defaultValues={{ network: networkValue(2) }}>
          {(control) => <NetworkSelectInput control={control} name="network" label="Network" />}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "Description",
    component: () => (
      <Networks>
        <FormStory<Values> defaultValues={{ network: networkValue(1) }}>
          {(control) => (
            <NetworkSelectInput
              control={control}
              name="network"
              label="Network"
              description="The network whose directory lists the stream."
            />
          )}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "Allow empty",
    component: () => (
      <Networks>
        <FormStory<Values> defaultValues={{ network: "" }}>
          {(control) => (
            <NetworkSelectInput control={control} name="network" label="Network" allowEmpty />
          )}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "Multi",
    component: () => (
      <Networks>
        <FormStory<Values> defaultValues={{ networks: [networkValue(1), networkValue(3)] }}>
          {(control) => (
            <NetworkSelectInput control={control} name="networks" label="Networks" isMulti />
          )}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "Error",
    component: () => (
      <Networks>
        <FormStory<Values> validate>
          {(control) => (
            <NetworkSelectInput control={control} name="network" label="Network" rules={required} />
          )}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <Networks>
        <FormStory<Values> defaultValues={{ network: networkValue(2) }}>
          {(control) => (
            <NetworkSelectInput control={control} name="network" label="Network" isDisabled />
          )}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "Menu open",
    component: () => (
      <Networks count={12}>
        <FormStory<Values> defaultValues={{ network: networkValue(2) }}>
          {(control) => (
            <NetworkSelectInput control={control} name="network" label="Network" menuIsOpen />
          )}
        </FormStory>
      </Networks>
    ),
  },
  {
    name: "No networks",
    component: () => (
      <Networks count={0}>
        <FormStory<Values>>
          {(control) => (
            <NetworkSelectInput control={control} name="network" label="Network" menuIsOpen />
          )}
        </FormStory>
      </Networks>
    ),
  },
];

export default stories;
