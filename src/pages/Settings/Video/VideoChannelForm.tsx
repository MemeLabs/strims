// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { SubmitHandler, useForm } from "react-hook-form";

import {
  Button,
  ButtonSet,
  CreatableSelectInput,
  InputError,
  NetworkSelectInput,
  SelectInput,
  SelectOption,
  TextAreaInput,
  TextInput,
} from "../../../components/Form";

export interface VideoChannelFormData {
  title: string;
  description: string;
  tags: Array<SelectOption<string>>;
  networkKey: string;
  themeColor: SelectOption<number>;
}

export interface VideoChannelFormProps {
  values?: VideoChannelFormData;
  onSubmit: SubmitHandler<VideoChannelFormData>;
  error: Error;
  loading: boolean;
  submitLabel: string;
}

export const themeColorOptions = [
  {
    value: 0x000000,
    label: "black",
  },
  {
    value: 0xff0000,
    label: "red",
  },
  {
    value: 0x00ff00,
    label: "green",
  },
  {
    value: 0x0000ff,
    label: "blue",
  },
];

const VideoChannelForm: React.FC<VideoChannelFormProps> = ({
  values,
  onSubmit,
  error,
  loading,
  submitLabel,
}) => {
  const { handleSubmit, control, formState } = useForm<VideoChannelFormData>({
    mode: "onBlur",
    defaultValues: {
      title: "",
      description: "",
      tags: [],
      networkKey: "",
      ...values,
    },
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating channel"} />}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Title is required",
          },
          maxLength: {
            value: 100,
            message: "Title too long",
          },
        }}
        label="Title"
        placeholder="Title"
        name="title"
      />
      <TextAreaInput
        control={control}
        rules={{
          maxLength: {
            value: 500,
            message: "Description too long",
          },
        }}
        label="Description"
        placeholder="Description"
        name="description"
      />
      <SelectInput
        control={control}
        options={themeColorOptions}
        name="themeColor"
        label="Theme Color"
      />
      <CreatableSelectInput control={control} name="tags" label="Tags" placeholder="Tags" />
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
        <Button disabled={loading || formState.isSubmitting || !formState.isDirty}>
          {submitLabel}
        </Button>
      </ButtonSet>
    </form>
  );
};

export default VideoChannelForm;
