import * as React from "react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";

import { InputError, TextInput } from "../components/Form";
import { MainLayout } from "../components/MainLayout";
import { useCall, useLazyCall } from "../contexts/Api";
import * as pb from "../lib/pb";

const BootstrapClientForm = ({
  onCreate,
}: {
  onCreate: (res: pb.CreateBootstrapClientResponse) => void;
}) => {
  const [{ error, loading }, createBootstrapClient] = useLazyCall("bootstrap", "createClient", {
    onComplete: onCreate,
  });
  const { register, handleSubmit, errors } = useForm({
    mode: "onBlur",
  });

  const onSubmit = (data) =>
    createBootstrapClient(
      new pb.CreateBootstrapClientRequest({
        websocketOptions: {
          url: data.url,
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
  bootstrapClients: pb.IBootstrapClient[];
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

    return (
      <div className="thing_list__item" key={bootstrapClient.id}>
        {i}
        <span>{bootstrapClient.websocketOptions.url}</span>
        <button onClick={handleDelete}>delete</button>
        <pre>{JSON.stringify(bootstrapClient, null, 2)}</pre>
      </div>
    );
  });
  return <div className="thing_list">{rows}</div>;
};

const BootstrapClientsPage = () => {
  const [bootstrapClientsRes, getBootstrapClients] = useCall("bootstrap", "listClients");

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
          <BootstrapClientForm onCreate={() => getBootstrapClients()} />
          <h1>BootstrapClients</h1>
          <h2>Recommended BootstrapClients</h2>
          <p>Manage your connected bootstrapClients</p>
          <BootstrapClientTable
            bootstrapClients={bootstrapClientsRes.value?.bootstrapClients}
            onDelete={() => getBootstrapClients()}
          />
        </main>
      </div>
    </MainLayout>
  );
};

export default BootstrapClientsPage;
