import React from "react";
import { SubmitHandler, useForm } from "react-hook-form";

import {
  Button,
  ButtonSet,
  InputError,
  NetworkSelectInput,
  TextInput,
} from "../../../components/Form";
import BackLink from "../BackLink";

export interface AutoseedRuleFormData {
  networkKey: string;
  swarmId: string;
  salt: string;
}

export interface AutoseedRuleFormProps {
  values?: AutoseedRuleFormData;
  onSubmit: SubmitHandler<AutoseedRuleFormData>;
  error: Error;
  loading: boolean;
  indexLinkVisible: boolean;
}

const AutoseedRuleForm: React.FC<AutoseedRuleFormProps> = ({
  values,
  onSubmit,
  error,
  loading,
  indexLinkVisible,
}) => {
  const { handleSubmit, control, formState } = useForm<AutoseedRuleFormData>({
    mode: "onBlur",
    defaultValues: {
      networkKey: "",
      swarmId: "",
      salt: "",
      ...values,
    },
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating channel"} />}
      {indexLinkVisible ? (
        <BackLink
          to="/settings/autoseed/rules"
          title="Rules"
          description="Some description of rules..."
        />
      ) : (
        <BackLink
          to="/settings/autoseed/config"
          title="Autoseed"
          description="Some description of autoseed..."
        />
      )}
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
          {values ? "Update Rule" : "Create Rule"}
        </Button>
      </ButtonSet>
    </form>
  );
};

export default AutoseedRuleForm;
