import base32Decode from "base32-decode";
import { Base64 } from "js-base64";
import React from "react";
import { useNavigate } from "react-router-dom";

import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import AutoseedRuleForm, { AutoseedRuleFormData } from "./AutoseedRuleForm";

const ChatModifierCreateFormPage: React.FC = () => {
  const [{ value }] = useCall("autoseed", "listRules");
  const navigate = useNavigate();
  const [{ error, loading }, createRule] = useLazyCall("autoseed", "createRule", {
    onComplete: () => navigate(`/settings/autoseed/rules`, { replace: true }),
  });

  const onSubmit = React.useCallback(async (data: AutoseedRuleFormData) => {
    await createRule({
      rule: {
        networkKey: Base64.toUint8Array(data.networkKey),
        swarmId: new Uint8Array(base32Decode(data.swarmId, "RFC4648")),
        salt: new TextEncoder().encode(data.salt),
      },
    });
  }, []);

  return (
    <AutoseedRuleForm
      onSubmit={onSubmit}
      error={error}
      loading={loading}
      indexLinkVisible={!!value?.rules.length}
    />
  );
};

export default ChatModifierCreateFormPage;
