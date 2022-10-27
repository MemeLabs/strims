// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { useForm } from "react-hook-form";

import { Button, ButtonSet, InputError, TextInput } from "../../../components/Form";
import { validNamePattern } from "../../../lib/validation";
import ForwardLink from "../ForwardLink";

export interface NetworkFormData {
  alias: string;
}

export interface NetworkFormProps {
  onSubmit: (data: NetworkFormData) => void;
  error: Error;
  loading: boolean;
  networkId: bigint;
  showDirectoryFormLink: boolean;
  values?: NetworkFormData;
}

const NetworkForm: React.FC<NetworkFormProps> = ({
  onSubmit,
  error,
  loading,
  networkId,
  showDirectoryFormLink,
  values,
}) => {
  const { control, handleSubmit } = useForm<{
    alias: string;
  }>({
    mode: "onBlur",
    defaultValues: {
      alias: "",
      ...values,
    },
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating membership"} />}
      <TextInput
        control={control}
        rules={{
          pattern: {
            value: validNamePattern,
            message: "Name contains invalid characters",
          },
        }}
        label="Alternate Name"
        name="alias"
        placeholder="Enter an alternate name for this network"
        description="Your name in chat and private messages."
      />
      <ButtonSet>
        <Button disabled={loading}>Update Network</Button>
      </ButtonSet>
      {showDirectoryFormLink && (
        <>
          <ForwardLink to={`/settings/networks/${networkId}/directory`} title="Directory" />
          <ForwardLink to={`/settings/networks/${networkId}/icon`} title="Icon" />
          <ForwardLink to={`/settings/networks/${networkId}/peers`} title="Peers" />
          <ForwardLink
            to={`/settings/networks/${networkId}/alias-reservations`}
            title="Alias reservations"
          />
        </>
      )}
    </form>
  );
};

export default NetworkForm;
