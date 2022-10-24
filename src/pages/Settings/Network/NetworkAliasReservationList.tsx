// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useMemo } from "react";
import { useTranslation } from "react-i18next";
import { Link, useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { AliasReservation } from "../../../apis/strims/network/v1/network";
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

  const sortedAliasReservations = useMemo(
    () =>
      aliasReservations ? aliasReservations.sort((a, b) => a.alias.localeCompare(b.alias)) : [],
    [aliasReservations]
  );

  if (!aliasReservations) {
    return null;
  }

  const handleResetCooldown = async (id: bigint) => {
    await client.network.resetAliasReservationCooldown({ id });
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
        <TableCell>
          <Link to={reservation.id.toString()}>{reservation.alias}</Link>
        </TableCell>
        <TableCell>{state}</TableCell>
        <MenuCell>
          {reservation.peerKey.length === 0 && (
            <MenuItem label="Reset cooldown" onClick={() => handleResetCooldown(reservation.id)} />
          )}
        </MenuCell>
      </tr>
    );
  });

  return (
    <>
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
  const [{ loading, value }, listAliasReservations] = useCall("network", "listAliasReservations", {
    args: [{ networkId: BigInt(networkId) }],
  });

  if (loading) {
    return null;
  }
  return (
    <>
      <TableTitleBar
        label="Network alias reservations"
        backLink={`/settings/networks/${networkId}`}
      />
      <AliasReservationTable
        aliasReservations={value.aliasReservations}
        onChange={listAliasReservations}
      />
    </>
  );
};

export default NetworkAliasReservationList;
