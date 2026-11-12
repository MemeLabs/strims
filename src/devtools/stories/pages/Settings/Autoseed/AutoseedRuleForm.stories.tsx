// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Base64 } from "js-base64";
import React, { useRef } from "react";

import AutoseedRuleForm, {
  AutoseedRuleFormData,
  AutoseedRuleFormProps,
} from "../../../../../pages/Settings/Autoseed/AutoseedRuleForm";
import NetworkService from "../../../../mocks/network/service";
import { MockApi, Story, fixtures } from "../../../../storybook";
import { SettingsContent, useSubmitOnLoad } from "../Layout.harness";

const filled: AutoseedRuleFormData = {
  label: "Speedrun relay",
  networkKey: Base64.fromUint8Array(fixtures.networkKey(2)),
  swarmId: "MFRGGZDFMZTWQ2LKNNWG23TPOBYXE43U",
  salt: "relay-1",
};

interface ContextProps extends Partial<AutoseedRuleFormProps> {
  submit?: boolean;
}

// The form in a settings page over five fixture networks; submitting it
// does nothing.
const Context: React.FC<ContextProps> = ({ submit, ...props }) => {
  const ref = useRef<HTMLDivElement>(null);
  useSubmitOnLoad(ref, submit);
  return (
    <MockApi services={() => ({ network: new NetworkService({ count: 5 }) })}>
      <SettingsContent ref={ref}>
        <AutoseedRuleForm
          onSubmit={() => undefined}
          error={null}
          loading={false}
          submitLabel="Create Rule"
          {...props}
        />
      </SettingsContent>
    </MockApi>
  );
};

const stories: Story[] = [
  {
    name: "New",
    component: () => <Context />,
  },
  {
    name: "Filled",
    component: () => <Context values={filled} submitLabel="Update Rule" />,
  },
  {
    // Submitted empty.
    name: "Invalid",
    component: () => <Context submit />,
  },
  {
    // Submitted with a swarm ID that isn't base32.
    name: "Invalid swarm ID",
    component: () => (
      <Context values={{ ...filled, swarmId: "not-base32!" }} submitLabel="Update Rule" submit />
    ),
  },
  {
    // The save's error, as the create and edit pages pass it.
    name: "Error",
    component: () => (
      <Context
        values={filled}
        submitLabel="Update Rule"
        error={new Error({ message: "a rule for this swarm exists" })}
      />
    ),
  },
];

export default stories;
