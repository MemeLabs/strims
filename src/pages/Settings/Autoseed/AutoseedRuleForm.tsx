import React from "react";
import { SubmitHandler, useForm } from "react-hook-form";

import {
  Button,
  ButtonSet,
  InputError,
  NetworkSelectInput,
  TextInput,
} from "../../../components/Form";

export interface AutoseedRuleFormData {
  label: string;
  networkKey: string;
  swarmId: string;
  salt: string;
}

export interface AutoseedRuleFormProps {
  values?: AutoseedRuleFormData;
  onSubmit: SubmitHandler<AutoseedRuleFormData>;
  error: Error;
  loading: boolean;
  submitLabel: string;
}

const AutoseedRuleForm: React.FC<AutoseedRuleFormProps> = ({
  values,
  onSubmit,
  error,
  loading,
  submitLabel,
}) => {
  const { handleSubmit, control, formState } = useForm<AutoseedRuleFormData>({
    mode: "onBlur",
    defaultValues: {
      label: "",
      networkKey: "",
      swarmId: "",
      salt: "",
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
            message: "Label is required",
          },
        }}
        label="Label"
        placeholder="Label"
        name="label"
      />
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
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Swarm ID is required",
          },
        }}
        label="Swarm ID"
        placeholder="Swarm ID"
        name="swarmId"
      />
      <TextInput control={control} label="Salt" placeholder="Salt" name="salt" />
      <ButtonSet>
        <Button disabled={loading || formState.isSubmitting || !formState.isDirty}>
          {submitLabel}
        </Button>
      </ButtonSet>
    </form>
  );
};

export default AutoseedRuleForm;
