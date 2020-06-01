import * as React from "react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";
import { InputError, TextInput } from "../components/Form";
import { MainLayout } from "../components/MainLayout";
import { useCall, useLazyCall, useClient } from "../contexts/Api";
import * as pb from "../lib/pb";

const NetworkForm = ({ onCreate }: { onCreate: (res: pb.CreateNetworkResponse) => void }) => {
  const [{ value, error, loading }, createNetwork] = useLazyCall("createNetwork", { onComplete: onCreate });
  const { register, handleSubmit, errors } = useForm({
    mode: "onBlur",
  });

  const onSubmit = (data) => createNetwork(new pb.CreateNetworkRequest(data));

  return (
    <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
      {error && <InputError error={error.message || "Error creating network"} />}
      <TextInput
        error={errors.name}
        inputRef={register({
          required: {
            value: true,
            message: "Name is required",
          },
          pattern: {
            value: /^\S+$/i,
            message: "Names contains invalid characers",
          },
        })}
        label="Name"
        name="name"
        placeholder="Enter a network name"
        required
      />
      <div className="input_buttons">
        <button className="input input_button" disabled={loading}>
          Create Network
        </button>
      </div>
    </form>
  );
};

const wrapString = (str: string, cols: number) =>
  new Array(Math.ceil(str.length / cols))
    .fill("")
    .map((_, i) => str.substr(i * cols, cols))
    .join("\n");

const unwrapString = (str: string) => str.replace(/\n/g, "");

const NetworkTable = ({ networks, onDelete }: { networks: pb.INetwork[]; onDelete: () => void }) => {
  const [, deleteNetwork] = useLazyCall("deleteNetwork", { onComplete: onDelete });
  const client = useClient();

  if (!networks) {
    return null;
  }

  const rows = networks.map((network, i) => {
    const handleDelete = () => deleteNetwork({ id: network.id });

    const handleCreateInvite = async () => {
      const invitation = await client.createNetworkInvitation({
        signingKey: network.key,
        signingCert: network.certificate,
        networkName: network.name,
      });
      navigator.clipboard.writeText(invitation.invitationB64);
      console.log("copied invite to clipboard");
    };

    return (
      <div className="thing_list__item" key={network.id}>
        {i}
        <span>{network.name}</span>
        <button onClick={handleDelete} className="input input_button">
          delete
        </button>
        <button onClick={handleCreateInvite} className="input input_button">
          create invite
        </button>
        <pre>{JSON.stringify(network, null, 2)}</pre>
      </div>
    );
  });
  return <div className="thing_list">{rows}</div>;
};

const NetworksPage = () => {
  const [networksRes, getNetworks] = useCall("getNetworks");

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
          <NetworkForm onCreate={() => getNetworks()} />
          <h1>Networks</h1>
          <h2>Recommended Networks</h2>
          <p>Manage your connected networks</p>
          <NetworkTable networks={networksRes.value?.networks} onDelete={() => getNetworks()} />
        </main>
      </div>
    </MainLayout>
  );
};

export default NetworksPage;
