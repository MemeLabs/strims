// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React, { useEffect, useMemo } from "react";
import { FormProvider, useForm } from "react-hook-form";

import {
  Button,
  ButtonSet,
  CreatableSelectInput,
  ImageInput,
  ImageValue,
  InputError,
  InputLabel,
  SelectInput,
  SelectOption,
  TextInput,
  ToggleInput,
} from "../../../components/Form";
import {
  ChatStyleSheetFormData,
  ChatStyleSheetInput,
} from "../../../components/Settings/ChatStyleSheet";
import { useCall } from "../../../contexts/FrontendApi";
import { ScaleOption, scaleOptions } from "./utils";

export interface ChatEmoteFormData extends ChatStyleSheetFormData {
  name: string;
  image: ImageValue;
  scale: ScaleOption;
  contributor: string;
  contributorLink: string;
  extraWrapCount: number;
  wrapAdjacent: boolean;
  animated: boolean;
  animationFrameCount: number;
  animationDuration: number;
  animationIterationCount: number;
  animationEndOnFrame: number;
  animationLoopForever: boolean;
  animationAlternateDirection: boolean;
  defaultModifiers: SelectOption<string>[];
  labels: SelectOption<string>[];
  enable: boolean;
}

export interface ChatEmoteFormProps {
  onSubmit?: (data: ChatEmoteFormData) => void;
  onChange?: (data: ChatEmoteFormData) => void;
  error?: Error;
  loading?: boolean;
  serverId?: bigint;
  values?: ChatEmoteFormData;
  submitLabel: string;
}

const noop = () => undefined;

const ChatEmoteForm: React.FC<ChatEmoteFormProps> = ({
  onSubmit = noop,
  onChange = noop,
  error = null,
  loading = false,
  serverId = BigInt(0),
  values = {},
  submitLabel,
}) => {
  const formMethods = useForm<ChatEmoteFormData>({
    mode: "onBlur",
    defaultValues: {
      scale: scaleOptions[0],
      ...values,
    },
  });
  const { handleSubmit, control, watch } = formMethods;

  if (onChange) {
    const values = watch();
    useEffect(() => onChange(values), [values]);
  }

  const animated = watch("animated");

  const [listModifiersRes] = useCall("chatServer", "listModifiers", { args: [{ serverId }] });
  const modifierOptions: SelectOption<string>[] = useMemo(
    () => listModifiersRes.value?.modifiers.map(({ name }) => ({ label: name, value: name })),
    [listModifiersRes.value]
  );

  const [listLabelsRes] = useCall("chatServer", "listEmoteLabels", { args: [{ serverId }] });
  const labelOptions: SelectOption<string>[] = useMemo(
    () => listLabelsRes.value?.labels.map((label) => ({ label, value: label })),
    [listLabelsRes.value]
  );

  return (
    <FormProvider {...formMethods}>
      <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
        {error && <InputError error={error.message || "Error creating chat server"} />}
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Name is required",
            },
          }}
          name="name"
          label="Name"
          placeholder="Enter a emote name"
        />
        <InputLabel required={true} text="Image" component="div">
          <ImageInput
            control={control}
            name="image"
            maxSize={10485764}
            rules={{
              required: {
                value: true,
                message: "Image is required",
              },
            }}
          />
        </InputLabel>
        <SelectInput
          control={control}
          name="scale"
          label="Scale"
          options={scaleOptions}
          isSearchable={false}
        />
        <TextInput control={control} label="contributor" name="contributor" />
        <TextInput control={control} label="contributor link" name="contributorLink" />

        <TextInput
          control={control}
          rules={{
            min: 0,
            max: {
              value: 10,
              message: "Rendering too many elements will degrade performance",
            },
          }}
          type="number"
          name="extraWrapCount"
          label="Extra Wrappers"
          placeholder="Enter a number of extra wrapper elements to render"
        />
        <ToggleInput control={control} label="wrap adjacent emotes" name="wrapAdjacent" />
        <ChatStyleSheetInput />
        <ToggleInput control={control} label="animated" name="animated" />
        <TextInput
          control={control}
          label="frame count"
          name="animationFrameCount"
          type="number"
          disabled={!animated}
        />
        <TextInput
          control={control}
          label="duration (ms)"
          name="animationDuration"
          type="number"
          disabled={!animated}
        />
        <TextInput
          control={control}
          label="loops"
          name="animationIterationCount"
          type="number"
          disabled={!animated}
        />
        <TextInput
          control={control}
          label="end on frame"
          name="animationEndOnFrame"
          type="number"
          disabled={!animated}
        />
        <ToggleInput
          control={control}
          label="loop forever"
          name="animationLoopForever"
          disabled={!animated}
        />
        <ToggleInput
          control={control}
          label="alternate directions"
          name="animationAlternateDirection"
          disabled={!animated}
        />
        <CreatableSelectInput
          control={control}
          name="defaultModifiers"
          label="Default modifiers"
          placeholder="Modifiers"
          options={modifierOptions}
        />
        <CreatableSelectInput
          control={control}
          name="labels"
          label="Labels"
          options={labelOptions}
        />
        <ToggleInput control={control} label="enable" name="enable" />
        <ButtonSet>
          <Button disabled={loading}>{submitLabel}</Button>
        </ButtonSet>
      </form>
    </FormProvider>
  );
};

export default ChatEmoteForm;
