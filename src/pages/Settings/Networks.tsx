import { Base64 } from "js-base64";
import React from "react";
import ReactDOM from "react-dom";
import { Controller, useForm } from "react-hook-form";
import { Link } from "react-router-dom";
import Select from "react-select";

import {
  CreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationResponse,
  CreateNetworkRequest,
  CreateNetworkResponse,
  Network,
} from "../../apis/strims/network/v1/network";
import { InputError, InputLabel, TextInput } from "../../components/Form";
import { MainLayout } from "../../components/MainLayout";
import { useCall, useClient, useLazyCall } from "../../contexts/FrontendApi";
import { useProfile } from "../../contexts/Profile";
import jsonutil from "../../lib/jsonutil";

const NetworkForm = ({ onCreate }: { onCreate: (res: CreateNetworkResponse) => void }) => {
  const [{ value, error, loading }, createNetwork] = useLazyCall("network", "create", {
    onComplete: onCreate,
  });
  const { register, handleSubmit, errors } = useForm({
    mode: "onBlur",
  });

  const onSubmit = (data) => createNetwork(new CreateNetworkRequest(data));

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

const JoinForm = ({
  onCreate,
}: {
  onCreate: (res: CreateNetworkFromInvitationResponse) => void;
}) => {
  const [{ value, error, loading }, create] = useLazyCall("network", "createFromInvitation", {
    onComplete: onCreate,
  });
  const { register, handleSubmit, errors } = useForm<{
    invitationB64: string;
  }>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit(({ invitationB64 }) =>
    create(
      new CreateNetworkFromInvitationRequest({
        invitation: { invitationB64 },
      })
    )
  );

  return (
    <form className="invite_form" onSubmit={onSubmit}>
      {error && <InputError error={error.message || "Error creating membership"} />}
      <TextInput
        error={errors.invitationB64}
        inputRef={register({
          required: {
            value: true,
            message: "invite is required",
          },
          pattern: {
            value: /^[a-zA-Z0-9+/]+={0,2}$/,
            message: "invalid invite string",
          },
        })}
        label="Invite string"
        name="invitationB64"
        placeholder="Enter an invite string"
        required
      />
      <div className="input_buttons">
        <button className="input input_button" disabled={loading}>
          Create Memberhip
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

const PublishNetworkModal = ({ network, onClose }: { network: Network; onClose: () => void }) => {
  const [bootstrapPeersRes] = useCall("bootstrap", "listPeers");
  const client = useClient();
  const { register, handleSubmit, errors, control } = useForm({
    mode: "onBlur",
  });

  const onSubmit = (data) => {
    console.log(data);
    client.bootstrap.publishNetworkToPeer({
      peerId: data.peer.value,
      network: network,
    });
    onClose();
  };

  if (bootstrapPeersRes.loading) {
    return null;
  }

  return ReactDOM.createPortal(
    <>
      <div className="thing_list__modal_mask"></div>
      <div className="thing_list__modal">
        <form className="thing_form" onSubmit={handleSubmit(onSubmit)}>
          <InputLabel required={true} text="Peer">
            <Controller
              as={Select}
              className="input_select"
              placeholder="Select peer"
              options={bootstrapPeersRes.value?.peers.map((p) => ({
                value: p.peerId,
                label: p.label,
              }))}
              name="peer"
              control={control}
              rules={{
                required: {
                  value: true,
                  message: "Network is required",
                },
              }}
            />
            <InputError error={errors.peer} />
          </InputLabel>
          <div className="input_buttons">
            <button className="input input_button" onClick={onClose}>
              Cancel
            </button>
            <button className="input input_button">Publish Network</button>
          </div>
        </form>
      </div>
    </>,
    document.body
  );
};

const NetworkTable = ({ networks, onDelete }: { networks: Network[]; onDelete: () => void }) => {
  const [, deleteNetwork] = useLazyCall("network", "delete", { onComplete: onDelete });
  const client = useClient();
  const [{ profile }] = useProfile();

  const [publishNetwork, setPublishNetwork] = React.useState<Network>();
  const modal = publishNetwork && (
    <PublishNetworkModal network={publishNetwork} onClose={() => setPublishNetwork(null)} />
  );

  if (!networks) {
    return null;
  }

  const rows = networks.map((network, i) => {
    const handleDelete = () => deleteNetwork({ id: network.id });

    const handleCreateInvite = async () => {
      const invitation = await client.network.createInvitation({
        signingKey: profile.key,
        signingCert: network.certificate,
        networkName: network.name,
      });
      navigator.clipboard.writeText(invitation.invitationB64);
      console.log("copied invite to clipboard");
    };

    const handlePublish = () => setPublishNetwork(network);

    return (
      <div className="thing_list__item" key={network.id.toString()}>
        {i}
        <span>{network.name}</span>
        <button onClick={handleDelete} className="input input_button">
          delete
        </button>
        <button onClick={handleCreateInvite} className="input input_button">
          create invite
        </button>
        <button onClick={handlePublish} className="input input_button">
          publish
        </button>
        <pre>{jsonutil.stringify(network)}</pre>
      </div>
    );
  });
  return (
    <div className="thing_list">
      {modal}
      {rows}
    </div>
  );
};

const NetworksPage = () => {
  const [networksRes, getNetworks] = useCall("network", "list");

  return (
    <main className="network_page">
      <NetworkForm onCreate={() => getNetworks()} />
      <JoinForm onCreate={() => getNetworks()} />
      <NetworkTable networks={networksRes.value?.networks} onDelete={() => getNetworks()} />
    </main>
  );
};

export default NetworksPage;
