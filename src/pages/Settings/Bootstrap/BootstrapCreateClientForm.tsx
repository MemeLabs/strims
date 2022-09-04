// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import BootstrapForm, { BootstrapFormData } from "./BootstrapClientForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.bootstrap.title"));

  const [{ value }] = useCall("bootstrap", "listClients");
  const navigate = useNavigate();
  const [{ error, loading }, createClient] = useLazyCall("bootstrap", "createClient", {
    onComplete: () => navigate(`/settings/bootstrap/clients`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: BootstrapFormData) => {
    await createClient({
      clientOptions: {
        websocketOptions: data,
      },
    });
  }, []);

  return (
    <>
      <TableTitleBar
        label="Create Bootstrap"
        backLink={!!value?.bootstrapClients.values.length && "/settings/bootstrap/clients"}
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
