// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { BootstrapClient } from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import BootstrapForm, { BootstrapFormData } from "./BootstrapClientForm";

const BootstrapEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.bootstrap.title"));

  const { clientId } = useParams<"clientId">();
  const [{ value, ...getRes }] = useCall("bootstrap", "getClient", {
    args: [{ id: BigInt(clientId) }],
  });

  const navigate = useNavigate();
  const [updateRes, updateBootstrap] = useLazyCall("bootstrap", "updateClient", {
    onComplete: () => navigate(`/settings/bootstrap/clients`),
  });

  const onSubmit = React.useCallback(async (data: BootstrapFormData) => {
    await updateBootstrap({
      id: BigInt(clientId),
      clientOptions: {
        websocketOptions: data,
      },
    });
  }, []);

  if (getRes.loading) {
    return null;
  }

  let data: BootstrapFormData;
  switch (value.bootstrapClient.clientOptions.case) {
    case BootstrapClient.ClientOptionsCase.WEBSOCKET_OPTIONS: {
      data = value.bootstrapClient.clientOptions.websocketOptions;
      break;
    }
    default:
      return null;
  }

  return (
    <>
      <TableTitleBar label="Edit Bootstrap" backLink="/settings/bootstrap/clients" />
      <BootstrapForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={data}
        submitLabel="Update Bootstrap"
      />
    </>
  );
};

export default BootstrapEditForm;
