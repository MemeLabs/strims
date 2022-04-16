import React from "react";
import { SubmitHandler, useForm } from "react-hook-form";

import { Button, ButtonSet, InputError, TextInput } from "../../../components/Form";

export interface BootstrapFormData {
  url: string;
}

export interface BootstrapFormProps {
  values?: BootstrapFormData;
  onSubmit: SubmitHandler<BootstrapFormData>;
  error: Error;
  loading: boolean;
  submitLabel: string;
}

const BootstrapForm: React.FC<BootstrapFormProps> = ({
  values,
  onSubmit,
  error,
  loading,
  submitLabel,
}) => {
  const { handleSubmit, control } = useForm<BootstrapFormData>({
    mode: "onBlur",
    defaultValues: {
      url: "",
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
            message: "URL is required",
          },
          pattern: {
            value: /^\S+$/i,
            message: "Invalid format",
          },
        }}
        autoCapitalize="off"
        autoCorrect="off"
        label="URL"
        name="url"
        placeholder="Enter a bootstrap url"
      />
      <ButtonSet>
        <Button disabled={loading}>{submitLabel}</Button>
      </ButtonSet>
    </form>
  );
};

export default BootstrapForm;
