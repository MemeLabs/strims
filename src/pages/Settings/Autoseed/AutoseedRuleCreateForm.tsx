// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import base32Decode from "base32-decode";
import { Base64 } from "js-base64";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import AutoseedRuleForm, { AutoseedRuleFormData } from "./AutoseedRuleForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.autoseed.title"));

  const [{ value }] = useCall("autoseed", "listRules");
  const navigate = useNavigate();
  const [{ error, loading }, createRule] = useLazyCall("autoseed", "createRule", {
    onComplete: () => navigate(`/settings/autoseed/rules`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: AutoseedRuleFormData) => {
    await createRule({
      rule: {
        label: data.label,
        networkKey: Base64.toUint8Array(data.networkKey),
        swarmId: new Uint8Array(base32Decode(data.swarmId, "RFC4648")),
        salt: new TextEncoder().encode(data.salt),
      },
    });
  }, []);

  const backLink = value?.rules.length ? "/settings/autoseed/rules" : "/settings/autoseed/config";

  return (
    <>
      <TableTitleBar label="Create Rule" backLink={backLink} />
      <AutoseedRuleForm
        onSubmit={onSubmit}
        error={error}
        loading={loading}
        submitLabel={"Create Rule"}
      />
    </>
  );
};

export default ChatModifierCreateFormPage;
