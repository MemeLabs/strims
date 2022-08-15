// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import base32Decode from "base32-decode";
import base32Encode from "base32-encode";
import { Base64 } from "js-base64";
import React from "react";
import { useTranslation } from "react-i18next";
import { useNavigate, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import AutoseedRuleForm, { AutoseedRuleFormData } from "./AutoseedRuleForm";

const AutoseedRuleEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.autoseed.title"));

  const { ruleId } = useParams<"ruleId">();
  const [{ value, ...getRes }] = useCall("autoseed", "getRule", {
    args: [{ id: BigInt(ruleId) }],
  });

  const navigate = useNavigate();
  const [updateRes, updateAutoseedRule] = useLazyCall("autoseed", "updateRule", {
    onComplete: () => navigate(`/settings/autoseed/rules`),
  });

  const onSubmit = React.useCallback(async (data: AutoseedRuleFormData) => {
    await updateAutoseedRule({
      id: BigInt(ruleId),
      rule: {
        label: data.label,
        networkKey: Base64.toUint8Array(data.networkKey),
        swarmId: new Uint8Array(base32Decode(data.swarmId, "RFC4648")),
        salt: new TextEncoder().encode(data.salt),
      },
    });
  }, []);

  if (getRes.loading) {
    return null;
  }

  const { rule } = value;
  const data: AutoseedRuleFormData = {
    label: rule.label,
    networkKey: Base64.fromUint8Array(rule.networkKey),
    swarmId: base32Encode(rule.swarmId, "RFC4648", { padding: false }),
    salt: new TextDecoder().decode(rule.salt),
  };

  return (
    <>
      <TableTitleBar label="Edit Rule" backLink="/settings/autoseed/rules" />
      <AutoseedRuleForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={data}
        submitLabel="Update Rule"
      />
    </>
  );
};

export default AutoseedRuleEditForm;
