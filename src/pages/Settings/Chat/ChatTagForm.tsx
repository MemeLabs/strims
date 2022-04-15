import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { useForm } from "react-hook-form";

import { Button, ButtonSet, InputError, TextInput, ToggleInput } from "../../../components/Form";

export interface ChatTagFormData {
  name: string;
  color: string;
  sensitive: boolean;
}

export interface ChatTagFormProps {
  onSubmit: (data: ChatTagFormData) => void;
  error: Error;
  loading: boolean;
  values?: ChatTagFormData;
  submitLabel: string;
}

const ChatTagForm: React.FC<ChatTagFormProps> = ({
  onSubmit,
  error,
  loading,
  values = {},
  submitLabel,
}) => {
  const { handleSubmit, control } = useForm<ChatTagFormData>({
    mode: "onBlur",
    defaultValues: values,
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating tag"} />}
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
        rules={{
          required: {
            value: true,
            message: "Color is required",
          },
        }}
        name="color"
        label="Color"
        placeholder="Enter a color code"
      />
      <ToggleInput control={control} name="sensitive" label="Sensitive" />
      <ButtonSet>
        <Button disabled={loading}>{submitLabel}</Button>
      </ButtonSet>
    </form>
  );
};

export default ChatTagForm;
