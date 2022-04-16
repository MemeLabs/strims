import React from "react";
import { useForm } from "react-hook-form";

import { Button, ButtonSet, InputError, TextInput } from "../../../components/Form";
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
            value: /^\S+$/i,
            message: "Name contains invalid characters",
          },
        }}
        label="Alternate Name"
        name="alias"
        placeholder="Enter an alternate name for this network"
      />
      <ButtonSet>
        <Button disabled={loading}>Update Network</Button>
      </ButtonSet>
      {showDirectoryFormLink && (
        <ForwardLink
          to={`/settings/networks/${networkId}/directory`}
          title="Directory"
          description="Directory embed settings..."
        />
      )}
    </form>
  );
};

export default NetworkForm;
