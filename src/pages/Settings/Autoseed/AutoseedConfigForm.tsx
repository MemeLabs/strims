// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { Config } from "../../../apis/strims/autoseed/v1/autoseed";
import { Button, ButtonSet, InputError, ToggleInput } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ForwardLink from "../ForwardLink";

interface AutoseedConfigFormData {
  enable: boolean;
}

const AutoseedConfigForm = () => {
  const { t } = useTranslation();
  useTitle(t("settings.autoseed.title"));

  const [setConfigRes, setConfig] = useLazyCall("autoseed", "setConfig");

  const { handleSubmit, reset, control, formState } = useForm<AutoseedConfigFormData>({
    mode: "onBlur",
    defaultValues: {
      enable: false,
    },
  });

  const setValues = ({ config }: { config?: Config }) =>
    reset(
      {
        enable: config.enable,
      },
      {
        keepDirty: false,
        keepIsValid: false,
      }
    );

  useCall("autoseed", "getConfig", { onComplete: (res) => setValues(res) });

  const onSubmit = handleSubmit(async (data) => {
    const res = await setConfig({
      config: {
        enable: data.enable,
      },
    });
    setValues(res);
  });

  return (
    <>
      <TableTitleBar label="Autoseed" />
      <form className="thing_form" onSubmit={onSubmit}>
        {setConfigRes.error && (
          <InputError error={setConfigRes.error.message || "Error saving autoseed settings"} />
        )}
        <ToggleInput control={control} label="Enable" name="enable" />
        <ButtonSet>
          <Button disabled={formState.isSubmitting || !formState.isDirty}>Save Changes</Button>
        </ButtonSet>
        <ForwardLink
          to={`/settings/autoseed/rules`}
          title="Autoseed rules"
          description="Some description of autoseed rules..."
        />
      </form>
    </>
  );
};

export default AutoseedConfigForm;
