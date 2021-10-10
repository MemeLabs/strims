import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Base64 } from "js-base64";
import React from "react";
import { useForm } from "react-hook-form";
import { MdChevronRight } from "react-icons/md";
import { Link } from "react-router-dom";
import { useAsync } from "react-use";

import { Server } from "../../../apis/strims/chat/v1/chat";
import { InputError, SelectInput, SelectOption, TextInput } from "../../../components/Form";
import { useClient } from "../../../contexts/FrontendApi";
import { certificateRoot } from "../../../lib/certificate";
import BackLink from "./BackLink";

export interface ChatServerFormData {
  name: string;
  networkKey: SelectOption<string>;
  tags: SelectOption<string>[];
  modifiers: SelectOption<string>[];
}

export interface ChatServerFormProps {
  onSubmit: (data: ChatServerFormData) => void;
  error: Error;
  loading: boolean;
  config?: Server;
  indexLinkVisible?: boolean;
}

const ChatServerForm: React.FC<ChatServerFormProps> = ({
  onSubmit,
  error,
  loading,
  config,
  indexLinkVisible,
}) => {
  const client = useClient();

  const { handleSubmit, control, reset } = useForm<ChatServerFormData>({
    mode: "onBlur",
  });

  const { value: networkOptions } = useAsync(async () => {
    const res = await client.network.list();
    return res.networks.map((n) => {
      const certRoot = certificateRoot(n.certificate);
      return {
        value: Base64.fromUint8Array(certRoot.key),
        label: certRoot.subject,
      };
    });
  });

  const setValues = (config: Server) => {
    const networkKey = Base64.fromUint8Array(config.networkKey);

    reset(
      {
        name: config.room.name,
        networkKey: networkOptions.find(({ value }) => value === networkKey),
      },
      {
        keepDirty: false,
        keepIsValid: false,
      }
    );
  };

  React.useEffect(() => {
    if (networkOptions && config) {
      setValues(config);
    }
  }, [networkOptions, config]);

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating chat server"} />}
      {indexLinkVisible && (
        <BackLink
          to="/settings/chat-servers"
          title="Servers"
          description="Some description of servers..."
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
        placeholder="Enter a chat room name"
      />
      <SelectInput
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
        options={networkOptions}
      />
      {config && (
        <>
          <Link
            className="input_label input_button"
            to={`/settings/chat-servers/${config.id}/emotes`}
          >
            <div className="input_label__body">
              <div>Emotes</div>
              <div>Some description of emotes...</div>
            </div>
            <MdChevronRight size="28" />
          </Link>
          <Link
            className="input_label input_button"
            to={`/settings/chat-servers/${config.id}/modifiers`}
          >
            <div className="input_label__body">
              <div>Emote modifiers</div>
              <div>Some description of emote modifiers...</div>
            </div>
            <MdChevronRight size="28" />
          </Link>
          <Link
            className="input_label input_button"
            to={`/settings/chat-servers/${config.id}/tags`}
          >
            <div className="input_label__body">
              <div>Tags</div>
              <div>Some description of tags...</div>
            </div>
            <MdChevronRight size="28" />
          </Link>
        </>
      )}
      <label className="input_label">
        <div className="input_label__body">
          <button className="input input_button" disabled={loading}>
            {config ? "Update Server" : "Create Server"}
          </button>
        </div>
      </label>
    </form>
  );
};

export default ChatServerForm;
