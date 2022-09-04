// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { SubmitHandler, useForm } from "react-hook-form";
import createUrlRegExp from "url-regex-safe";

import { Button, ButtonSet, InputError, TextInput, ToggleInput } from "../../../components/Form";

export interface BootstrapFormData {
  url: string;
  insecureSkipVerifyTls: boolean;
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
            value: createUrlRegExp(),
            message: "Invalid format",
          },
        }}
        autoCapitalize="off"
        autoCorrect="off"
        label="URL"
        name="url"
        placeholder="Enter a bootstrap url"
      />
      <ToggleInput
        control={control}
        name="insecureSkipVerifyTls"
        label="Skip TLS verification"
        description="Ignore invalid TLS certificates (native clients only)"
      />
      <ButtonSet>
        <Button disabled={loading}>{submitLabel}</Button>
      </ButtonSet>
    </form>
  );
};

export default BootstrapForm;
