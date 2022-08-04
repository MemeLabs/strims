// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import base32Encode from "base32-encode";
import React, { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { Link, Navigate } from "react-router-dom";
import { useTitle } from "react-use";

import { Rule } from "../../../apis/strims/autoseed/v1/autoseed";
import {
  MenuCell,
  MenuItem,
  MenuLink,
  Table,
  TableCell,
  TableMenu,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";

interface AutoseedRuleTableItemProps {
  rule: Rule;
  onDelete: () => void;
}

const AutoseedRuleTableItem = ({ rule, onDelete }: AutoseedRuleTableItemProps) => {
  const [, deleteRule] = useLazyCall("autoseed", "deleteRule", {
    onComplete: onDelete,
  });

  const handleDelete = useCallback(() => deleteRule({ id: rule.id }), [rule]);

  return (
    <tr>
      <td>
        <Link to={`/settings/autoseed/rules/${rule.id}`}>{rule.label}</Link>
      </td>
      <TableCell truncate>{base32Encode(rule.swarmId, "RFC4648", { padding: false })}</TableCell>
      <MenuCell>
        <MenuItem label="Delete" onClick={handleDelete} />
      </MenuCell>
    </tr>
  );
};

const AutoseedRulesList = () => {
  const { t } = useTranslation();
  useTitle(t("settings.autoseed.title"));

  const [rulesRes, listRules] = useCall("autoseed", "listRules");

  if (rulesRes.loading) {
    return null;
  }
  if (!rulesRes.value?.rules.length) {
    return <Navigate to="/settings/autoseed/rules/new" />;
  }

  const rows = rulesRes.value?.rules?.map((rule) => {
    return <AutoseedRuleTableItem key={rule.id.toString()} rule={rule} onDelete={listRules} />;
  });

  return (
    <>
      <TableTitleBar label="Autoseed Rules" backLink="/settings/autoseed/config">
        <TableMenu label="Create">
          <MenuLink label="Create Rule" to="/settings/autoseed/rules/new" />
        </TableMenu>
      </TableTitleBar>
      <Table>
        <thead>
          <tr>
            <th>Label</th>
            <th>ID</th>
            <th></th>
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </Table>
    </>
  );
};

export default AutoseedRulesList;
