import * as React from "react";
import { Controller, useForm } from "react-hook-form";
import { Link } from "react-router-dom";
import Select from "react-select";

import { InputError, InputLabel, TextInput } from "../components/Form";
import { MainLayout } from "../components/MainLayout";
import { useCall, useLazyCall } from "../contexts/Api";
import * as pb from "../lib/pb";

const ChatServerForm = ({ onCreate }: { onCreate: (res: pb.CreateChatServerResponse) => void }) => {
  const [{ value, error, loading }, createChatServer] = useLazyCall("createChatServer", {
    onComplete: onCreate,
  });
  const [networkMembershipsRes] = useCall("getNetworkMemberships");

  const { register, handleSubmit, control, errors } = useForm({
    mode: "onBlur",
  });

  const onSubmit = (data) => {
    createChatServer(
      new pb.CreateChatServerRequest({
        networkKey: data.networkKey.value,
        chatRoom: {
          name: data.name,
        },
      })
    );
  };

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
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
          options={networkMembershipsRes.value?.networkMemberships.map((n) => ({
            value: n.caCertificate.key,
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
  servers: pb.IChatServer[];
  onDelete: () => void;
}) => {
  const [, deleteChatServer] = useLazyCall("deleteChatServer", { onComplete: onDelete });

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
  const [serversRes, getChatServers] = useCall("getChatServers");

  return (
    <MainLayout>
      <div>
        <Link className="settings_link" to="/networks">
          Networks
        </Link>
        <Link className="settings_link" to="/memberships">
          Network Memberships
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
