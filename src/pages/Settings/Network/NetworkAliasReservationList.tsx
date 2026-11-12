// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { AliasReservation } from "../../../apis/strims/network/v1/network";
import { InputError } from "../../../components/Form";
import {
  MenuCell,
  MenuItem,
  Table,
  TableCell,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useClient } from "../../../contexts/FrontendApi";

interface NetworkTableProps {
  aliasReservations: AliasReservation[];
  onChange: () => void;
}

const AliasReservationTable: React.FC<NetworkTableProps> = ({ aliasReservations, onChange }) => {
  const client = useClient();
  const [error, setError] = useState<string>(null);

  const sortedAliasReservations = useMemo(
    () =>
      aliasReservations ? aliasReservations.sort((a, b) => a.alias.localeCompare(b.alias)) : [],
    [aliasReservations]
  );

  if (!aliasReservations) {
    return null;
  }

  // The list reloads after a reset; a failure shows above the table until
  // the next reset.
  const handleResetCooldown = async ({ id, alias }: AliasReservation) => {
    setError(null);
    try {
      await client.network.resetAliasReservationCooldown({ id });
    } catch (e) {
      setError(`Error resetting ${alias}'s cooldown: ${(e as Error).message || "unknown error"}`);
      return;
    }
    onChange();
  };

  const rows = sortedAliasReservations.map((reservation) => {
    let state = "Active";
    if (reservation.peerKey.length === 0) {
      state = "On cooldown";
      if (Number(reservation.reservedUntil < Date.now() / 1000)) {
        state = "Free";
      }
    }

    return (
      <tr key={reservation.id.toString()}>
        <TableCell truncate>
          <Link to={reservation.id.toString()}>{reservation.alias}</Link>
        </TableCell>
        <TableCell>{state}</TableCell>
        <MenuCell>
          {reservation.peerKey.length === 0 && (
            <MenuItem
              label="Reset cooldown"
              onClick={() => void handleResetCooldown(reservation)}
            />
          )}
        </MenuCell>
      </tr>
    );
  });

  return (
    <>
      {error && (
        <div className="thing_form">
          <InputError error={error} />
        </div>
      )}
      <Table>
        <thead>
          <tr>
            <th>Name</th>
            <th>State</th>
            <th />
          </tr>
        </thead>
        <tbody>{rows}</tbody>
      </Table>
    </>
  );
};

const NetworkAliasReservationList: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const { networkId } = useParams<"networkId">();
  const [{ loading, value, error }, listAliasReservations] = useCall(
    "network",
    "listAliasReservations",
    {
      args: [{ networkId: BigInt(networkId) }],
    }
  );

  if (loading) {
    return null;
  }
  return (
    <>
      <TableTitleBar
        label="Network alias reservations"
        backLink={`/settings/networks/${networkId}`}
      />
      {error ? (
        <div className="thing_form">
          <InputError error={error.message || "Error loading alias reservations"} />
        </div>
      ) : (
        <AliasReservationTable
          aliasReservations={value.aliasReservations}
          onChange={listAliasReservations}
        />
      )}
    </>
  );
};

export default NetworkAliasReservationList;
