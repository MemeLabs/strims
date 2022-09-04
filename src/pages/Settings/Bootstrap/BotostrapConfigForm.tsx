// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { Config, SetConfigResponse } from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import { Button, ButtonSet, InputError, ToggleInput } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ForwardLink from "../ForwardLink";

interface BootstrapConfigFormProps {
  config: Config;
  onCreate: (res: SetConfigResponse) => void;
}

const BootstrapConfigForm = ({ onCreate, config }: BootstrapConfigFormProps) => {
  const [{ error, loading }, setConfig] = useLazyCall("bootstrap", "setConfig", {
    onComplete: onCreate,
  });
  const { control, handleSubmit } = useForm<{
    enablePublishing: boolean;
  }>({
    mode: "onBlur",
    defaultValues: {
      enablePublishing: config.enablePublishing,
    },
  });

  const onSubmit = handleSubmit((data) =>
    setConfig({
      config: {
        enablePublishing: data.enablePublishing,
      },
    })
  );

  return (
    <form className="thing_form" onSubmit={onSubmit}>
      {error && <InputError error={error.message || "Error saving config"} />}
      <ToggleInput control={control} label="Enable publishing" name="enablePublishing" />
      <ButtonSet>
        <Button disabled={loading}>Store Config</Button>
      </ButtonSet>
      <ForwardLink
        to={`/settings/bootstrap/clients`}
        title="Bootstrap clients"
        description="Some description of bootstrap clients..."
      />
    </form>
  );
};

const BootstrapConfigPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.bootstrap.title"));

  const [{ value }, getConfig] = useCall("bootstrap", "getConfig");

  return (
    <>
      <TableTitleBar label="Edit Bootstrap Config" />
      {value && <BootstrapConfigForm config={value.config} onCreate={() => getConfig()} />}
    </>
  );
};

export default BootstrapConfigPage;
