import * as React from "react";
import { Controller, useForm } from "react-hook-form";
import { Link } from "react-router-dom";
import Select, { OptionTypeBase } from "react-select";

import { InputError, InputLabel, TextInput } from "../components/Form";
import { MainLayout } from "../components/MainLayout";
import { useCall, useLazyCall } from "../contexts/Api";
import {
  CreateChatServerRequest,
  CreateChatServerResponse,
  ICertificate,
  IChatServer,
} from "../lib/pb";

interface ChatServerFormData {
  name: string;
  networkKey: {
    value: Uint8Array;
    label: string;
  };
}

const rootCertificate = (cert: ICertificate): ICertificate =>
  cert.parent ? rootCertificate(cert.parent) : cert;

const ChatServerForm = ({ onCreate }: { onCreate: (res: CreateChatServerResponse) => void }) => {
  const [{ error, loading }, createChatServer] = useLazyCall("chat", "createServer", {
    onComplete: onCreate,
  });
  const [networksRes] = useCall("network", "list");

  const { register, handleSubmit, control, errors } = useForm<ChatServerFormData>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit((data) => {
    createChatServer(
      new CreateChatServerRequest({
        networkKey: data.networkKey.value,
        chatRoom: {
          name: data.name,
        },
      })
    );
  });

  return (
    <form className="thing_form" onSubmit={onSubmit}>
      {error && <InputError error={error.message || "Error creating chat server"} />}
      <TextInput
        error={errors?.name}
        inputRef={register({
          required: {
            value: true,
            message: "Name is required",
          },
        })}
        label="Name"
        name="name"
        placeholder="Enter a chat room name"
        required
      />
      <InputLabel required={true} text="Network">
        <Controller
          as={Select}
          className="input_select"
          placeholder="Select network"
          options={networksRes.value?.networks.map((n) => ({
            value: rootCertificate(n.certificate).key,
            label: n.name,
          }))}
          name="networkKey"
          control={control}
          rules={{
            required: {
              value: true,
              message: "Network is required",
            },
          }}
        />
        <InputError error={errors.networkKey} />
      </InputLabel>
      <div className="input_buttons">
        <button className="input input_button" disabled={loading}>
          Create ChatServer
        </button>
      </div>
    </form>
  );
};

const ChatServerTable = ({
  servers,
  onDelete,
}: {
  servers: IChatServer[];
  onDelete: () => void;
}) => {
  const [, deleteChatServer] = useLazyCall("chat", "deleteServer", { onComplete: onDelete });

  if (!servers) {
    return null;
  }

  const rows = servers.map((server, i) => {
    const handleDelete = () => deleteChatServer({ id: server.id });

    return (
      <div className="thing_list__item" key={server.id}>
        <span>{server.chatRoom.name}</span>
        <button className="input input_button" onClick={handleDelete}>
          delete
        </button>
        <pre>{JSON.stringify(server, null, 2)}</pre>
      </div>
    );
  });
  return <div className="thing_list">{rows}</div>;
};

const ChatServersPage = () => {
  const [serversRes, getChatServers] = useCall("chat", "listServers");

  return (
    <MainLayout>
      <div className="page_body">
        <Link className="settings_link" to="/networks">
          Networks
        </Link>
        <Link className="settings_link" to="/bootstrap-clients">
          Bootstrap Clients
        </Link>
        <Link className="settings_link" to="/chat-servers">
          Chat Servers
        </Link>
        <main className="network_page">
          <ChatServerForm onCreate={() => getChatServers()} />
          <h1>Chat Servers</h1>
          <p>Manage your chat servers</p>
          <ChatServerTable
            servers={serversRes.value?.chatServers}
            onDelete={() => getChatServers()}
          />
        </main>
      </div>
    </MainLayout>
  );
};

export default ChatServersPage;
