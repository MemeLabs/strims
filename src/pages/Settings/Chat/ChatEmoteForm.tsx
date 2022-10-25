// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React, { useMemo } from "react";
import { useForm } from "react-hook-form";

import { EmoteScale } from "../../../apis/strims/chat/v1/chat";
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
  TextAreaInput,
  TextInput,
  ToggleInput,
} from "../../../components/Form";
import { useCall } from "../../../contexts/FrontendApi";
import { ScaleOption, scaleOptions } from "./utils";

export interface ChatEmoteFormData {
  name: string;
  image: ImageValue;
  scale: ScaleOption;
  contributor: string;
  contributorLink: string;
  css: string;
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
  onSubmit: (data: ChatEmoteFormData) => void;
  error?: Error;
  loading?: boolean;
  serverId?: bigint;
  values?: ChatEmoteFormData;
  submitLabel: string;
}

const ChatEmoteForm: React.FC<ChatEmoteFormProps> = ({
  onSubmit,
  error = null,
  loading = false,
  serverId = BigInt(0),
  values = {},
  submitLabel,
}) => {
  const { handleSubmit, control, watch } = useForm<ChatEmoteFormData>({
    mode: "onBlur",
    defaultValues: {
      scale: scaleOptions[0],
      ...values,
    },
  });

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
      <TextAreaInput control={control} label="css" name="css" />
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
      <CreatableSelectInput control={control} name="labels" label="Labels" options={labelOptions} />
      <ToggleInput control={control} label="enable" name="enable" />
      <ButtonSet>
        <Button disabled={loading}>{submitLabel}</Button>
      </ButtonSet>
    </form>
  );
};

export default ChatEmoteForm;
