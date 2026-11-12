// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { InputError } from "../../../components/Form";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { fromFormImageValue, toFormImageValue } from "../../../lib/image";
import NetworkIconForm, { NetworkIconFormData } from "./NetworkIconForm";

const NetworkIconEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const { networkId } = useParams<"networkId">();
  const [{ value, ...getRes }] = useCall("network", "get", { args: [{ id: BigInt(networkId) }] });

  const network = value?.network;

  const navigate = useNavigate();
  const [updateRes, updateServerConfig] = useLazyCall("network", "updateServerConfig", {
    onComplete: () => navigate(`/settings/networks/${networkId}`),
  });

  if (getRes.loading) {
    return null;
  }
  if (!network) {
    return (
      <>
        <TableTitleBar label="Edit Network" backLink={`/settings/networks/${networkId}`} />
        <div className="thing_form">
          <InputError error={getRes.error?.message || "Error loading network"} />
        </div>
      </>
    );
  }
  if (!network.serverConfig) {
    return null;
  }

  const { serverConfig } = network;

  const onSubmit = async (data: NetworkIconFormData) => {
    const icon = fromFormImageValue(data.image);
    try {
      await updateServerConfig({
        networkId: BigInt(networkId),
        serverConfig: {
          ...serverConfig,
          icon,
        },
      });
    } catch {
      // The form shows updateRes.error.
    }
  };

  const data: NetworkIconFormData = {
    image: serverConfig.icon ? toFormImageValue(serverConfig.icon) : null,
  };

  return (
    <>
      <TableTitleBar label="Edit Network" backLink={`/settings/networks/${network.id}`} />
      <NetworkIconForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={data}
        submitLabel={"Update Icon"}
      />
    </>
  );
};

export default NetworkIconEditForm;
