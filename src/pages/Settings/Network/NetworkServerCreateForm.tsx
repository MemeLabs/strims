import React from "react";
import { useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";

import { Button, ButtonSet, InputError, TextInput } from "../../../components/Form";
import { useLazyCall } from "../../../contexts/FrontendApi";

const CreateForm: React.FC = () => {
  const navigate = useNavigate();
  const [{ error, loading }, createNetwork] = useLazyCall("network", "createServer", {
    onComplete: () => navigate("/settings/networks"),
  });
  const { control, handleSubmit } = useForm<{
    name: string;
    alias: string;
  }>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit((data) => createNetwork(data));

  return (
    <form className="thing_form" onSubmit={onSubmit}>
      {error && <InputError error={error.message || "Error creating network"} />}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Name is required",
          },
          pattern: {
            value: /^\S+$/i,
            message: "Names contains invalid characers",
          },
        }}
        label="Name"
        name="name"
        placeholder="Enter a network name"
      />
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
        <Button disabled={loading}>Create Network</Button>
      </ButtonSet>
    </form>
  );
};

export default CreateForm;
