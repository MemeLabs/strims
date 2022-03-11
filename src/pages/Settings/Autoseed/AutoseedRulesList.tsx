import base32Encode from "base32-encode";
import React from "react";
import { Link, Navigate } from "react-router-dom";

import { Rule } from "../../../apis/strims/autoseed/v1/autoseed";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import jsonutil from "../../../lib/jsonutil";
import BackLink from "../BackLink";

interface AutoseedRuleTableItemProps {
  rule: Rule;
  onDelete: () => void;
}

const AutoseedRuleTableItem = ({ rule, onDelete }: AutoseedRuleTableItemProps) => {
  return (
    <div className="thing_list__item">
      <div>
        <Link to={`/settings/autoseed/rules/${rule.id}`}>
          {base32Encode(rule.swarmId, "RFC4648", { padding: false })}
        </Link>
      </div>
      <button className="input input_button" onClick={onDelete}>
        delete
      </button>
      <pre>{jsonutil.stringify(rule)}</pre>
    </div>
  );
};

const AutoseedRulesList = () => {
  const [rulesRes, listRules] = useCall("autoseed", "listRules");
  const [, deleteRule] = useLazyCall("autoseed", "deleteRule", {
    onComplete: listRules,
  });

  if (rulesRes.loading) {
    return null;
  }
  if (!rulesRes.value?.rules.length) {
    return <Navigate to="/settings/autoseed/rules/new" />;
  }

  const rows = rulesRes.value?.rules?.map((rule) => {
    return (
      <AutoseedRuleTableItem
        key={rule.id.toString()}
        rule={rule}
        onDelete={() => deleteRule({ id: rule.id })}
      />
    );
  });

  return (
    <>
      <Link to="/settings/autoseed/rules/new">Create rule</Link>
      <div className="thing_list">
        <BackLink
          to={`/settings/autoseed/config`}
          title="Autoseed"
          description="Some description of autoseed..."
        />
        {rows}
      </div>
    </>
  );
};

export default AutoseedRulesList;
