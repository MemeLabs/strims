// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { HLSEgressConfig } from "../../../apis/strims/video/v1/hls_egress";
import { Button, ButtonSet, InputError, TextInput, ToggleInput } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { isAddrLength, isHttpUrl } from "../../../lib/address";

interface HLSEgressConfigFormData {
  enabled: boolean;
  serverAddr: string;
  publicServerAddr: string;
  serviceNetworkKeys: string[];
}

const HLSEgressConfigForm = () => {
  const { t } = useTranslation();
  useTitle(t("settings.videoEgress.title"));

  const [setConfigRes, setConfig] = useLazyCall("hlsEgress", "setConfig");

  const { handleSubmit, reset, control, formState } = useForm<HLSEgressConfigFormData>({
    mode: "onBlur",
    defaultValues: {
      enabled: false,
      serverAddr: "",
      publicServerAddr: "",
      serviceNetworkKeys: [],
    },
  });

  const setValues = ({ config }: { config?: HLSEgressConfig }) =>
    reset(
      {
        enabled: config.enabled,
        publicServerAddr: config.publicServerAddr,
      },
      {
        keepDirty: false,
        keepIsValid: false,
      }
    );

  useCall("hlsEgress", "getConfig", { onComplete: (res) => setValues(res) });

  const onSubmit = handleSubmit(async (data) => {
    try {
      const res = await setConfig({
        config: {
          enabled: data.enabled,
          publicServerAddr: data.publicServerAddr,
        },
      });
      setValues(res);
    } catch {
      // The form shows setConfigRes.error.
    }
  });

  return (
    <>
      <TableTitleBar label="Video Egress" />
      <form className="thing_form" onSubmit={onSubmit}>
        {setConfigRes.error && (
          <InputError error={setConfigRes.error.message || "Error saving ingress settings"} />
        )}
        <ToggleInput control={control} label="Enable" name="enabled" />
        <TextInput
          control={control}
          rules={{
            validate: {
              length: (v: string) => isAddrLength(v) || "Public address is too long",
              url: (v: string) =>
                !v ||
                isHttpUrl(v) ||
                "Public address must be an http:// or https:// URL, e.g. https://cdn.strims.gg",
            },
          }}
          label="Public address"
          description="Public address where clients can reach the HLS server."
          name="publicServerAddr"
          placeholder="ex: https://cdn.strims.gg"
        />
        <ButtonSet>
          <Button disabled={formState.isSubmitting || !formState.isDirty}>Save Changes</Button>
        </ButtonSet>
      </form>
    </>
  );
};

export default HLSEgressConfigForm;
