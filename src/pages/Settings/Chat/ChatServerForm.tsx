// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import React from "react";
import { useForm } from "react-hook-form";

import {
  Button,
  ButtonSet,
  InputError,
  NetworkSelectInput,
  TextInput,
} from "../../../components/Form";
import ForwardLink from "../ForwardLink";

export interface ChatServerFormData {
  name: string;
  networkKey: string;
}

export interface ChatServerFormProps {
  onSubmit: (data: ChatServerFormData) => void;
  error: Error;
  loading: boolean;
  id?: bigint;
  values?: ChatServerFormData;
  submitLabel: string;
}

const ChatServerForm: React.FC<ChatServerFormProps> = ({
  onSubmit,
  error,
  loading,
  id,
  values,
  submitLabel,
}) => {
  const { handleSubmit, control, formState } = useForm<ChatServerFormData>({
    mode: "onBlur",
    defaultValues: values,
  });

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating chat server"} />}
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
        placeholder="Enter a chat room name"
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
      {id && (
        <>
          <ForwardLink
            to={`/settings/chat-servers/${id}/icon`}
            title="Server Icon"
            description="Some description of server icon..."
          />
          <ForwardLink
            to={`/settings/chat-servers/${id}/emotes`}
            title="Emotes"
            description="Some description of emotes..."
          />
          <ForwardLink
            to={`/settings/chat-servers/${id}/modifiers`}
            title="Emote modifiers"
            description="Some description of emote modifiers..."
          />
          <ForwardLink
            to={`/settings/chat-servers/${id}/tags`}
            title="Tags"
            description="Some description of tags..."
          />
        </>
      )}
      <ButtonSet>
        <Button disabled={loading || formState.isSubmitting || !formState.isDirty}>
          {submitLabel}
        </Button>
      </ButtonSet>
    </form>
  );
};

export default ChatServerForm;
