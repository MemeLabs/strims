import React from "react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";

import {
  BootstrapClient,
  CreateBootstrapClientRequest,
  CreateBootstrapClientResponse,
} from "../../apis/strims/network/v1/bootstrap/bootstrap";
import { InputError, TextInput } from "../../components/Form";
import { MainLayout } from "../../components/MainLayout";
import { useCall, useLazyCall } from "../../contexts/FrontendApi";
import jsonutil from "../../lib/jsonutil";

const BootstrapClientForm = ({
  onCreate,
}: {
  onCreate: (res: CreateBootstrapClientResponse) => void;
}) => {
  const [{ error, loading }, createBootstrapClient] = useLazyCall("bootstrap", "createClient", {
    onComplete: onCreate,
  });
  const { register, handleSubmit, errors } = useForm({
    mode: "onBlur",
  });

  const onSubmit = (data) =>
    createBootstrapClient(
      new CreateBootstrapClientRequest({
        clientOptions: {
          websocketOptions: {
            url: data.url,
          },
        },
      })
    );

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating bootstrapClient"} />}
      <TextInput
        error={errors.url && "URL is required"}
        inputRef={register({
          required: true,
          pattern: /^\S+$/i,
        })}
        label="URL"
        name="url"
        placeholder="Enter a bootstrap url"
        required
      />
      <div className="input_buttons">
        <button className="input input_button" disabled={loading}>
          Create BootstrapClient
        </button>
      </div>
    </form>
  );
};

const BootstrapClientTable = ({
  bootstrapClients,
  onDelete,
}: {
  bootstrapClients: BootstrapClient[];
  onDelete: () => void;
}) => {
  const [, deleteBootstrapClient] = useLazyCall("bootstrap", "deleteClient", {
    onComplete: onDelete,
  });

  if (!bootstrapClients) {
    return null;
  }

  const rows = bootstrapClients.map((bootstrapClient, i) => {
    const handleDelete = () => deleteBootstrapClient({ id: bootstrapClient.id });

    switch (bootstrapClient.clientOptions.case) {
      case BootstrapClient.ClientOptionsCase.WEBSOCKET_OPTIONS:
        return (
          <div className="thing_list__item" key={bootstrapClient.id.toString()}>
            {i}
            <span>{bootstrapClient.clientOptions.websocketOptions.url}</span>
            <button onClick={handleDelete}>delete</button>
            <pre>{jsonutil.stringify(bootstrapClient)}</pre>
          </div>
        );
      default:
        return (
          <div className="thing_list__item" key={bootstrapClient.id.toString()}>
            unknown bootstrap client type
          </div>
        );
    }
  });
  return <div className="thing_list">{rows}</div>;
};

const BootstrapClientsPage = () => {
  const [bootstrapClientsRes, getBootstrapClients] = useCall("bootstrap", "listClients");

  return (
    <main className="network_page">
      <BootstrapClientForm onCreate={() => getBootstrapClients()} />
      <h1>BootstrapClients</h1>
      <h2>Recommended BootstrapClients</h2>
      <p>Manage your connected bootstrapClients</p>
      <BootstrapClientTable
        bootstrapClients={bootstrapClientsRes.value?.bootstrapClients}
        onDelete={() => getBootstrapClients()}
      />
    </main>
  );
};

export default BootstrapClientsPage;
