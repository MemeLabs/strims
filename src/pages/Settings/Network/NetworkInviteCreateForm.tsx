// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { useState } from "react";
import { useForm } from "react-hook-form";
import { useNavigate, useParams } from "react-router";

import { BootstrapClient } from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import { Invitation } from "../../../apis/strims/network/v1/network";
import { Notification } from "../../../apis/strims/notification/v1/notification";
import { Button, ButtonSet, SelectInput, SelectOption } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useClient } from "../../../contexts/FrontendApi";
import { useNotification } from "../../../contexts/Notification";

const NetworkInviteCreateForm: React.FC = () => {
  const client = useClient();
  const { pushTransientNotification } = useNotification();
  const navigate = useNavigate();
  const { networkId } = useParams<"networkId">();

  const { control, handleSubmit, formState } = useForm<{
    bootstrapClient: SelectOption<bigint>;
  }>({
    mode: "onBlur",
  });

  const [options, setOptions] = useState<SelectOption<bigint>[]>();
  useCall("bootstrap", "listClients", {
    onComplete: (res) => {
      const options: SelectOption<bigint>[] = [];
      for (const client of res.bootstrapClients) {
        switch (client.clientOptions.case) {
          case BootstrapClient.ClientOptionsCase.WEBSOCKET_OPTIONS:
            options.push({
              label: client.clientOptions.websocketOptions.url,
              value: client.id,
            });
            break;
        }
        setOptions(options);
      }
    },
  });

  const onSubmit = handleSubmit(async ({ bootstrapClient }) => {
    const res = await client.network.createInvitation({
      networkId: BigInt(networkId),
      bootstrapClientId: bootstrapClient?.value ?? BigInt(0),
    });
    const invitation = Invitation.encode(res.invitation).finish();
    const code = Base64.fromUint8Array(invitation).replace(/={0,2}$/, "");
    void navigator.clipboard.writeText(code);
    pushTransientNotification({
      status: Notification.Status.STATUS_SUCCESS,
      message: "Invitation code copied to clipboard",
    });
    navigate("/settings/networks");
  });

  return (
    <>
      <TableTitleBar label="Create Invite" backLink={"/settings/networks"} />
      <form className="thing_form" onSubmit={onSubmit}>
        <SelectInput
          control={control}
          name="bootstrapClient"
          label="Bootstrap"
          placeholder="Select bootstrap"
          options={options}
        />
        <ButtonSet>
          <Button disabled={formState.isSubmitting}>Create Invite</Button>
        </ButtonSet>
      </form>
    </>
  );
};

export default NetworkInviteCreateForm;
