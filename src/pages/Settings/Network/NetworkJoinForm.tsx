// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useTitle } from "react-use";

import { Button, ButtonSet, InputError, TextInput } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { validNamePattern } from "../../../lib/validation";

const JoinForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const navigate = useNavigate();
  const [{ value }] = useCall("network", "list");
  const [{ error, loading }, create] = useLazyCall("network", "createNetworkFromInvitation", {
    onComplete: () => navigate("/settings/networks"),
  });
  const { control, handleSubmit } = useForm<{
    invitationB64: string;
    alias: string;
  }>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit(({ alias, invitationB64 }) =>
    create({
      alias,
      invitation: { invitationB64 },
    })
  );

  return (
    <>
      <TableTitleBar
        label="Join Network"
        backLink={!!value?.networks.length && "/settings/networks"}
      />
      <form className="thing_form" onSubmit={onSubmit}>
        {error && <InputError error={error.message || "Error creating membership"} />}
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "invite is required",
            },
            pattern: {
              value: /^[a-zA-Z0-9+/]+={0,2}$/,
              message: "invalid invite string",
            },
          }}
          label="Invite string"
          name="invitationB64"
          placeholder="Enter an invite string"
        />
        <TextInput
          control={control}
          rules={{
            pattern: {
              value: validNamePattern,
              message: "Name contains invalid characters",
            },
          }}
          label="Alternate Name"
          name="alias"
          placeholder="Enter an alternate name for this network"
        />
        <ButtonSet>
          <Button disabled={loading}>Join Network</Button>
        </ButtonSet>
      </form>
    </>
  );
};

export default JoinForm;
