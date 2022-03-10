import base32Decode from "base32-decode";
import base32Encode from "base32-encode";
import { Base64 } from "js-base64";
import React from "react";
import { useNavigate, useParams } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import AutoseedRuleForm, { AutoseedRuleFormData } from "./AutoseedRuleForm";

const AutoseedRuleEditForm: React.FC = () => {
  const { ruleId } = useParams<"ruleId">();
  const [{ value, ...getRes }] = useCall("autoseed", "getRule", {
    args: [{ id: BigInt(ruleId) }],
  });

  const navigate = useNavigate();
  const [updateRes, updateAutoseedRule] = useLazyCall("autoseed", "updateRule", {
    onComplete: () => navigate(`/settings/autoseed/rules`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: AutoseedRuleFormData) => {
    await updateAutoseedRule({
      id: BigInt(ruleId),
      rule: {
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
    networkKey: Base64.fromUint8Array(rule.networkKey),
    swarmId: base32Encode(rule.swarmId, "RFC4648", { padding: false }),
    salt: new TextDecoder().decode(rule.salt),
  };

  return (
    <AutoseedRuleForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      values={data}
      indexLinkVisible={true}
    />
  );
};

export default AutoseedRuleEditForm;
