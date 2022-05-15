// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { BootstrapClient } from "../../../apis/strims/network/v1/bootstrap/bootstrap";
import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import BootstrapForm, { BootstrapFormData } from "./BootstrapForm";

const BootstrapEditForm: React.FC = () => {
  const { ruleId } = useParams<"ruleId">();
  const [{ value, ...getRes }] = useCall("bootstrap", "getClient", {
    args: [{ id: BigInt(ruleId) }],
  });

  const navigate = useNavigate();
  const [updateRes, updateBootstrap] = useLazyCall("bootstrap", "updateClient", {
    onComplete: () => navigate(`/settings/bootstrap/rules`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: BootstrapFormData) => {
    await updateBootstrap({
      id: BigInt(ruleId),
      clientOptions: {
        websocketOptions: {
          url: data.url,
        },
      },
    });
  }, []);

  if (getRes.loading) {
    return null;
  }

  let data: BootstrapFormData;
  switch (value.bootstrapClient.clientOptions.case) {
    case BootstrapClient.ClientOptionsCase.WEBSOCKET_OPTIONS:
      data = {
        url: value.bootstrapClient.clientOptions.websocketOptions.url,
      };
      break;
    default:
      return null;
  }

  return (
    <>
      <TableTitleBar label="Edit Bootstrap" backLink="/settings/bootstraps" />
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
