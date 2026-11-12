// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router";
import { useTitle } from "react-use";

import { InputError } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import NetworkForm, { NetworkFormData } from "./NetworkForm";

const NetworkEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const { networkId } = useParams<"networkId">();
  const [{ value, ...getRes }] = useCall("network", "get", { args: [{ id: BigInt(networkId) }] });

  const network = value?.network;

  const navigate = useNavigate();
  const [updateRes, updateAlias] = useLazyCall("network", "updateAlias", {
    onComplete: () => navigate("/settings/networks"),
  });

  if (getRes.loading) {
    return null;
  }
  if (!network) {
    return (
      <>
        <TableTitleBar label="Edit Network" backLink="/settings/networks" />
        <div className="thing_form">
          <InputError error={getRes.error?.message || "Error loading network"} />
        </div>
      </>
    );
  }

  const onSubmit = async (data: NetworkFormData) => {
    try {
      await updateAlias({
        id: network.id,
        alias: data.alias,
      });
    } catch {
      // The form shows updateRes.error.
    }
  };

  const data: NetworkFormData = {
    alias: network.alias,
  };

  return (
    <>
      <TableTitleBar label="Edit Network" backLink="/settings/networks" />
      <NetworkForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        networkId={network.id}
        showDirectoryFormLink={!!network.serverConfig}
        values={data}
      />
    </>
  );
};

export default NetworkEditForm;
