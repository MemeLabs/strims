import React from "react";
import { useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";

import { InputError, TextInput } from "../../../components/Form";
import { useLazyCall } from "../../../contexts/FrontendApi";

const JoinForm: React.FC = () => {
  const navigate = useNavigate();
  const [{ error, loading }, create] = useLazyCall("network", "createNetworkFromInvitation", {
    onComplete: () => navigate("/settings/networks"),
  });
  const { control, handleSubmit } = useForm<{
    invitationB64: string;
    alias: string;
  }>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit(({ alias, invitationB64 }) =>
    create({
      alias,
      invitation: { invitationB64 },
    })
  );

  return (
    <form className="thing_form" onSubmit={onSubmit}>
      {error && <InputError error={error.message || "Error creating membership"} />}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "invite is required",
          },
          pattern: {
            value: /^[a-zA-Z0-9+/]+={0,2}$/,
            message: "invalid invite string",
          },
        }}
        label="Invite string"
        name="invitationB64"
        placeholder="Enter an invite string"
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
      <div className="input_buttons">
        <button className="input input_button" disabled={loading}>
          Create Memberhip
        </button>
      </div>
    </form>
  );
};

export default JoinForm;
