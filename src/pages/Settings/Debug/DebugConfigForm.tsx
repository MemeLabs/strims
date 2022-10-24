// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { Config } from "../../../apis/strims/debug/v1/debug";
import {
  Button,
  ButtonSet,
  InputError,
  NetworkSelectInput,
  ToggleInput,
} from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import ForwardLink from "../ForwardLink";

interface DebugConfigFormData {
  enableMockStreams: boolean;
  mockStreamNetworkKey: string;
}

const DebugConfigForm = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  const [setConfigRes, setConfig] = useLazyCall("debug", "setConfig");

  const { handleSubmit, reset, control, formState } = useForm<DebugConfigFormData>({
    mode: "onBlur",
    defaultValues: {
      enableMockStreams: false,
    },
  });

  const setValues = ({ config }: { config?: Config }) =>
    reset(
      {
        enableMockStreams: config.enableMockStreams,
        mockStreamNetworkKey: Base64.fromUint8Array(config.mockStreamNetworkKey),
      },
      {
        keepDirty: false,
        keepIsValid: false,
      }
    );

  useCall("debug", "getConfig", { onComplete: (res) => setValues(res) });

  const onSubmit = handleSubmit(async (data) => {
    const res = await setConfig({
      config: {
        enableMockStreams: data.enableMockStreams,
        mockStreamNetworkKey: Base64.toUint8Array(data.mockStreamNetworkKey),
      },
    });
    setValues(res);
  });

  return (
    <>
      <TableTitleBar label="Debug" />
      <form className="thing_form" onSubmit={onSubmit}>
        {setConfigRes.error && (
          <InputError error={setConfigRes.error.message || "Error saving debug settings"} />
        )}
        <ToggleInput control={control} label="Enable mock streams" name="enableMockStreams" />
        <NetworkSelectInput
          control={control}
          allowEmpty
          name="mockStreamNetworkKey"
          label="Mock stream network"
          placeholder="Select network"
        />
        <ButtonSet>
          <Button disabled={formState.isSubmitting || !formState.isDirty}>Save Changes</Button>
        </ButtonSet>
        <ForwardLink to={`/settings/debug/mock-stream`} title="Mock streams" />
      </form>
    </>
  );
};

export default DebugConfigForm;
