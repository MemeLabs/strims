// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import ReactDOM from "react-dom";
import { useForm } from "react-hook-form";

import { Network } from "../../../apis/strims/network/v1/network";
import { SelectInput } from "../../../components/Form";
import { useCall, useClient } from "../../../contexts/FrontendApi";

const PublishNetworkModal = ({ network, onClose }: { network: Network; onClose: () => void }) => {
  const [bootstrapPeersRes] = useCall("bootstrap", "listPeers");
  const client = useClient();
  const { handleSubmit, control } = useForm<{
    peer: {
      value: bigint;
      label: string;
    };
  }>({
    mode: "onBlur",
  });

  const onSubmit = handleSubmit((data) => {
    void client.bootstrap.publishNetworkToPeer({
      peerId: data.peer.value,
      networkId: network.id,
    });
    onClose();
  });

  if (bootstrapPeersRes.loading) {
    return null;
  }

  return ReactDOM.createPortal(
    <>
      <div className="thing_list__modal_mask"></div>
      <div className="thing_list__modal">
        <form className="thing_form" onSubmit={onSubmit}>
          <SelectInput
            control={control}
            rules={{
              required: {
                value: true,
                message: "Network is required",
              },
            }}
            name="peer"
            label="Network"
            placeholder="Select network"
            options={bootstrapPeersRes.value?.peers.map((p) => ({
              value: p.peerId,
              label: p.label,
            }))}
          />
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

export default PublishNetworkModal;
