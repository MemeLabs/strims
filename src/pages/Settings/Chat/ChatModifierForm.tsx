// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { useForm } from "react-hook-form";

import { Button, ButtonSet, InputError, TextInput, ToggleInput } from "../../../components/Form";

export interface ChatModifierFormData {
  name: string;
  priority: number;
  internal: boolean;
  extraWrapCount: number;
}

export interface ChatModifierFormProps {
  onSubmit: (data: ChatModifierFormData) => void;
  error: Error;
  loading: boolean;
  values?: ChatModifierFormData;
  submitLabel: string;
}

const ChatModifierForm: React.FC<ChatModifierFormProps> = ({
  onSubmit,
  error,
  loading,
  values = {},
  submitLabel,
}) => {
  const { handleSubmit, control } = useForm<ChatModifierFormData>({
    mode: "onBlur",
    defaultValues: values,
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating modifier"} />}
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
        placeholder="Enter a modifier name"
      />
      <TextInput
        control={control}
        type="number"
        name="priority"
        label="Priority"
        placeholder="Enter a modifier priority"
      />
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
      <ToggleInput control={control} name="internal" label="Internal" />
      <ButtonSet>
        <Button disabled={loading}>{submitLabel}</Button>
      </ButtonSet>
    </form>
  );
};

export default ChatModifierForm;
