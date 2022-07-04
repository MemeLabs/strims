// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useNavigate } from "react-router-dom";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import BootstrapForm, { BootstrapFormData } from "./BootstrapForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const [{ value }] = useCall("bootstrap", "listClients");
  const navigate = useNavigate();
  const [{ error, loading }, createClient] = useLazyCall("bootstrap", "createClient", {
    onComplete: () => navigate(`/settings/bootstraps`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: BootstrapFormData) => {
    await createClient({
      clientOptions: {
        websocketOptions: {
          url: data.url,
        },
      },
    });
  }, []);

  return (
    <>
      <TableTitleBar
        label="Create Bootstrap"
        backLink={!!value?.bootstrapClients.values.length && "/settings/bootstraps"}
      />
      <BootstrapForm
        onSubmit={onSubmit}
        error={error}
        loading={loading}
        submitLabel="Create Bootstrap"
      />
    </>
  );
};

export default ChatModifierCreateFormPage;
