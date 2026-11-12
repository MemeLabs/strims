// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import ReactDOM from "react-dom";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { Network } from "../../../apis/strims/network/v1/network";
import { InputError, SelectInput } from "../../../components/Form";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { useLayout } from "../../../contexts/Layout";

const PublishNetworkModal = ({ network, onClose }: { network: Network; onClose: () => void }) => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const [bootstrapPeersRes] = useCall("bootstrap", "listPeers");
  const [{ error, loading }, publishNetworkToPeer] = useLazyCall(
    "bootstrap",
    "publishNetworkToPeer",
    { onComplete: () => onClose() }
  );
  const layout = useLayout();
  const { handleSubmit, control } = useForm<{
    peer: {
      value: bigint;
      label: string;
    };
  }>({
    mode: "onBlur",
  });

  // The modal closes once the network is published.
  const onSubmit = handleSubmit(async (data) => {
    try {
      await publishNetworkToPeer({
        peerId: data.peer.value,
        networkId: network.id,
      });
    } catch {
      // The form shows error.
    }
  });

  if (bootstrapPeersRes.loading) {
    return null;
  }

  return ReactDOM.createPortal(
    <>
      <div className="thing_list__modal_mask"></div>
      <div className="thing_list__modal">
        <form className="thing_form" onSubmit={onSubmit}>
          {error && <InputError error={error.message || "Error publishing the network"} />}
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
            <button type="button" className="input input_button" onClick={onClose}>
              Cancel
            </button>
            <button className="input input_button" disabled={loading}>
              Publish Network
            </button>
          </div>
        </form>
      </div>
    </>,
    layout?.root ?? document.body
  );
};

export default PublishNetworkModal;
