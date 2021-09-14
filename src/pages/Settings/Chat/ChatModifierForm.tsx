import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { useForm } from "react-hook-form";

import { InputError, TextInput, ToggleInput } from "../../../components/Form";
import BackLink from "./BackLink";

export interface ChatModifierFormData {
  name: string;
  priority: number;
  internal: boolean;
}

export interface ChatModifierFormProps {
  onSubmit: (data: ChatModifierFormData) => void;
  error: Error;
  loading: boolean;
  serverId: bigint;
  values?: ChatModifierFormData;
  indexLinkVisible?: boolean;
}

const ChatModifierForm: React.FC<ChatModifierFormProps> = ({
  onSubmit,
  error,
  loading,
  values = {},
  serverId,
  indexLinkVisible,
}) => {
  const { handleSubmit, control } = useForm<ChatModifierFormData>({
    mode: "onBlur",
    defaultValues: values,
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating modifier"} />}
      {indexLinkVisible ? (
        <BackLink
          to={`/settings/chat-servers/${serverId}/modifiers`}
          title="Modifiers"
          description="Some description of modifiers..."
        />
      ) : (
        <BackLink
          to={`/settings/chat-servers/${serverId}`}
          title="Server"
          description="Some description of server..."
        />
      )}
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
        type="number"
        name="priority"
        label="Priority"
        placeholder="Enter a modifier priority"
      />
      <ToggleInput control={control} name="internal" label="Internal" />
      <label className="input_label">
        <div className="input_label__body">
          <button className="input input_button" disabled={loading}>
            {values ? "Update Modifier" : "Create Modifier"}
          </button>
        </div>
      </label>
    </form>
  );
};

export default ChatModifierForm;
