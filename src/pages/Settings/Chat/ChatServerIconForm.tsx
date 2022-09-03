// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { useForm } from "react-hook-form";

import {
  Button,
  ButtonSet,
  ImageInput,
  ImageValue,
  InputError,
  InputLabel,
} from "../../../components/Form";

export interface ChatServerIconFormData {
  image: ImageValue;
}

export interface ChatServerIconFormProps {
  onSubmit: (data: ChatServerIconFormData) => void;
  error: Error;
  loading: boolean;
  values?: ChatServerIconFormData;
  submitLabel: string;
}

const ChatServerIconForm: React.FC<ChatServerIconFormProps> = ({
  onSubmit,
  error,
  loading,
  values = {},
  submitLabel,
}) => {
  const { handleSubmit, control } = useForm<ChatServerIconFormData>({
    mode: "onBlur",
    defaultValues: values,
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error saving icon"} />}
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
      <ButtonSet>
        <Button disabled={loading}>{submitLabel}</Button>
      </ButtonSet>
    </form>
  );
};

export default ChatServerIconForm;
