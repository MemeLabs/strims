// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import {
  Button,
  ButtonSet,
  InputError,
  NetworkSelectInput,
  SelectInput,
  SelectOption,
  TextInput,
} from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useLazyCall } from "../../../contexts/FrontendApi";

const timoeutMsOptions = [
  {
    value: 60000,
    label: "1 minute",
  },
  {
    value: 300000,
    label: "5 minutes",
  },
  {
    value: 600000,
    label: "10 minutes",
  },
  {
    value: 1800000,
    label: "30 minutes",
  },
  {
    value: 3600000,
    label: "1 hour",
  },
];

interface DebugMockStreamData {
  bitrateKbps: number;
  segmentIntervalMs: number;
  timeoutMs: SelectOption<number>;
  networkKey: string;
}

const DebugMockStream = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  const [startMockStreamRes, startMockStream] = useLazyCall("debug", "startMockStream");

  const { handleSubmit, control, formState } = useForm<DebugMockStreamData>({
    mode: "onBlur",
    defaultValues: {
      bitrateKbps: 6000,
      segmentIntervalMs: 1000,
      timeoutMs: timoeutMsOptions[0],
    },
  });

  const onSubmit = handleSubmit(async (data) => {
    return startMockStream({
      bitrateKbps: data.bitrateKbps,
      segmentIntervalMs: data.segmentIntervalMs,
      timeoutMs: data.timeoutMs.value,
      networkKey: Base64.toUint8Array(data.networkKey),
    });
  });

  return (
    <>
      <TableTitleBar label="Mock stream" backLink="/settings/debug" />
      <form className="thing_form" onSubmit={onSubmit}>
        {startMockStreamRes.error && (
          <InputError error={startMockStreamRes.error.message || "Error creating mock stream"} />
        )}
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Bitrate is required",
            },
          }}
          name="bitrateKbps"
          label="Bitrate (kbps)"
          type="number"
        />
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Segment interval is required",
            },
          }}
          name="segmentIntervalMs"
          label="Segment interval (ms)"
          type="number"
        />
        <SelectInput
          control={control}
          label="Duration"
          name="timeoutMs"
          options={timoeutMsOptions}
        />
        <NetworkSelectInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Network is required",
            },
          }}
          name="networkKey"
          label="Network"
          placeholder="Select network"
        />
        <ButtonSet>
          <Button disabled={formState.isSubmitting}>Start mock stream</Button>
        </ButtonSet>
      </form>
    </>
  );
};

export default DebugMockStream;
