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
import { validNamePattern, validNetworkNamePattern } from "../../../lib/validation";

const CreateForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const navigate = useNavigate();
  const [{ value }] = useCall("network", "list");
  const [{ error, loading }, createNetwork] = useLazyCall("network", "createServer", {
    onComplete: (res) => navigate(`/settings/networks/${res.network.id}`, { replace: true }),
  });
  const { control, handleSubmit } = useForm<{
    name: string;
    alias: string;
  }>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit((data) => createNetwork(data));

  return (
    <>
      <TableTitleBar
        label="Create Network"
        backLink={!!value?.networks.length && "/settings/networks"}
      />
      <form className="thing_form" onSubmit={onSubmit}>
        {error && <InputError error={error.message || "Error creating network"} />}
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Name is required",
            },
            pattern: {
              value: validNetworkNamePattern,
              message: "Names contains invalid characers",
            },
          }}
          label="Name"
          name="name"
          placeholder="Enter a network name"
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
          <Button disabled={loading}>Create Network</Button>
        </ButtonSet>
      </form>
    </>
  );
};

export default CreateForm;
