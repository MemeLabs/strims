import * as React from "react";
import { useForm } from "react-hook-form";
import { Link } from "react-router-dom";
import { InputError, TextInput } from "../components/Form";
import { MainLayout } from "../components/MainLayout";
import { useCall, useLazyCall } from "../contexts/Api";
import * as pb from "../lib/pb";

const NetworkTable = ({ networks, onDelete }: { networks: pb.INetworkMembership[]; onDelete: () => void }) => {
  const [, deleteNetworkMembership] = useLazyCall("deleteNetworkMembership", { onComplete: onDelete });

  if (!networks) {
    return null;
  }

  const rows = networks.map((network, i) => {
    const handleDelete = () => deleteNetworkMembership({ id: network.id });

    return (
      <div className="thing_list__item" key={network.id}>
        {i}
        <span>{network.name}</span>
        <button onClick={handleDelete}>delete</button>
        <pre>{JSON.stringify(network, null, 2)}</pre>
      </div>
    );
  });
  return <div className="thing_list">{rows}</div>;
};

const NetworkMembershipsPage = () => {
  const [networkMembershipsRes, getNetworkMemberships] = useCall("getNetworkMemberships");

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
          <h1>Network Membership</h1>
          <h2>Recommended Networks</h2>
          <p>Manage your connected networks</p>
          <NetworkTable
            networks={networkMembershipsRes.value?.networkMemberships}
            onDelete={() => getNetworkMemberships()}
          />
        </main>
      </div>
    </MainLayout>
  );
};

export default NetworkMembershipsPage;
