import base32Encode from "base32-encode";
import React from "react";
import { MdChevronLeft } from "react-icons/md";
import { Link, Navigate } from "react-router-dom";

import { Rule } from "../../../apis/strims/autoseed/v1/autoseed";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import jsonutil from "../../../lib/jsonutil";

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
        <Link className="input_label input_label--button" to="/settings/autoseed">
          <MdChevronLeft size="28" />
          <div className="input_label__body">
            <div>Autoseed</div>
            <div>Some description of autoseed...</div>
          </div>
        </Link>
        {rows}
      </div>
    </>
  );
};

export default AutoseedRulesList;
