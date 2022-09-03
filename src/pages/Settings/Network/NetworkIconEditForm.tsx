// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

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

  if (getRes.loading || !network.serverConfig) {
    return null;
  }

  const { serverConfig } = network;

  const onSubmit = (data: NetworkIconFormData) =>
    updateServerConfig({
      networkId: BigInt(networkId),
      serverConfig: {
        ...serverConfig,
        icon: fromFormImageValue(data.image),
      },
    });

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
