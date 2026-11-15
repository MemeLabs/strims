// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import { Base64 } from "js-base64";
import React, { useState } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { useTitle } from "react-use";

import { BootstrapClient_ClientOptionsCase } from "../../../apis/strims/network/v1/bootstrap/bootstrap.pb";
import { CreateInvitationResponse, Invitation } from "../../../apis/strims/network/v1/network.pb";
import { Notification_Status } from "../../../apis/strims/notification/v1/notification.pb";
import { Button, ButtonSet, InputError, SelectInput, SelectOption } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useClient } from "../../../contexts/FrontendApi";
import { useNotification } from "../../../contexts/Notification";
import { formatError } from "../../../lib/rpcError";

const NetworkInviteCreateForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

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
  const [error, setError] = useState<Error>();
  useCall("bootstrap", "listClients", {
    onComplete: (res) => {
      const options: SelectOption<bigint>[] = [];
      for (const client of res.bootstrapClients) {
        switch (client.clientOptions.case) {
          case BootstrapClient_ClientOptionsCase.WEBSOCKET_OPTIONS:
            options.push({
              label: client.clientOptions.websocketOptions.url,
              value: client.id,
            });
            break;
        }
      }
      setOptions(options);
    },
  });

  const onSubmit = handleSubmit(async ({ bootstrapClient }) => {
    let res: CreateInvitationResponse;
    try {
      res = await client.network.createInvitation({
        networkId: BigInt(networkId),
        bootstrapClientId: bootstrapClient?.value ?? BigInt(0),
      });
    } catch (e) {
      setError(e as Error);
      return;
    }
    const invitation = Invitation.encode(res.invitation).finish();
    const code = Base64.fromUint8Array(invitation).replace(/={0,2}$/, "");
    void navigator.clipboard.writeText(code);
    pushTransientNotification({
      status: Notification_Status.STATUS_SUCCESS,
      message: "Invitation code copied to clipboard",
    });
    void navigate("/settings/networks");
  });

  return (
    <>
      <TableTitleBar label="Create Invite" backLink={"/settings/networks"} />
      <form className="thing_form" onSubmit={onSubmit}>
        {error && <InputError error={formatError(error) || "Error creating invitation"} />}
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
