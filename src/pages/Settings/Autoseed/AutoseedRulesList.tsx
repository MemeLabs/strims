// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Error } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import base32Encode from "base32-encode";
import React, { useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, Navigate } from "react-router";
import { useTitle } from "react-use";

import { Rule } from "../../../apis/strims/autoseed/v1/autoseed";
import { InputError } from "../../../components/Form";
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
  onDeleteError: (error: Error) => void;
}

const AutoseedRuleTableItem = ({ rule, onDelete, onDeleteError }: AutoseedRuleTableItemProps) => {
  const [, deleteRule] = useLazyCall("autoseed", "deleteRule", {
    onComplete: onDelete,
    onError: onDeleteError,
  });

  const handleDelete = async () => {
    try {
      await deleteRule({ id: rule.id });
    } catch {
      // The list shows error.
    }
  };

  return (
    <tr>
      <TableCell>
        <Link to={`/settings/autoseed/rules/${rule.id}`}>{rule.label}</Link>
      </TableCell>
      <TableCell truncate>{base32Encode(rule.swarmId, "RFC4648", { padding: false })}</TableCell>
      <MenuCell>
        <MenuItem label="Delete" onClick={() => void handleDelete()} />
      </MenuCell>
    </tr>
  );
};

const AutoseedRulesList = () => {
  const { t } = useTranslation();
  useTitle(t("settings.autoseed.title"));

  const [rulesRes, listRules] = useCall("autoseed", "listRules");
  // A failed delete's error, until a delete succeeds.
  const [deleteError, setDeleteError] = useState<Error>(null);
  const handleDelete = () => {
    setDeleteError(null);
    void listRules();
  };

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
        onDelete={handleDelete}
        onDeleteError={setDeleteError}
      />
    );
  });

  return (
    <>
      <TableTitleBar label="Autoseed Rules" backLink="/settings/autoseed/config">
        <TableMenu label="Create">
          <MenuLink label="Create Rule" to="/settings/autoseed/rules/new" />
        </TableMenu>
      </TableTitleBar>
      {deleteError && (
        <div className="thing_form">
          <InputError error={deleteError.message || "Error deleting the rule"} />
        </div>
      )}
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
