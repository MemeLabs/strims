// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useCallback } from "react";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { Device } from "../../../apis/strims/profile/v1/profile";
import { Checkpoint } from "../../../apis/strims/replication/v1/replication";
import { InputError } from "../../../components/Form";
import {
  MenuCell,
  MenuItem,
  Table,
  TableCell,
  TableHeader,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";

interface DeviceTableItemProps {
  device: Device;
  checkpoint: Checkpoint;
  onDelete: () => void;
  current?: boolean;
}

const DeviceTableItem = ({ device, checkpoint, onDelete, current }: DeviceTableItemProps) => {
  const [, deleteClient] = useLazyCall("profile", "deleteDevice", {
    onComplete: onDelete,
  });

  const handleDelete = useCallback(() => deleteClient({ id: device.id }), [device]);

  console.log(checkpoint);

  return (
    <tr>
      <TableCell showFor="md">{device.id.toString()}</TableCell>
      <TableCell>{device.os}</TableCell>
      <TableCell showFor="sm">{device.device}</TableCell>
      <TableCell>{new Date(Number(device.lastLogin) * 1000).toLocaleString()}</TableCell>
      {current ? (
        <TableCell />
      ) : (
        <MenuCell>
          <MenuItem label="Delete" onClick={handleDelete} />
        </MenuCell>
      )}
    </tr>
  );
};

const DevicesList = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  const [devicesRes, listDevices] = useCall("profile", "listDevices");
  const [checkpointsRes, listCheckpointsRes] = useCall("replication", "listCheckpoints");

  const handleDelete = useCallback(() => {
    void listDevices();
    void listCheckpointsRes();
  }, [listDevices, listCheckpointsRes]);

  if (devicesRes.loading || checkpointsRes.loading) {
    return null;
  }
  const error = devicesRes.error || checkpointsRes.error;
  if (error) {
    return (
      <>
        <TableTitleBar label="Profile" backLink="/settings/profile" />
        <div className="thing_form">
          <InputError error={error.message || "Error loading devices"} />
        </div>
      </>
    );
  }

  const rows = devicesRes.value?.devices?.map((device) => {
    return (
      <DeviceTableItem
        key={device.id.toString()}
        device={device}
        checkpoint={checkpointsRes.value.checkpoints.find(({ id }) => id === device.id)}
        onDelete={handleDelete}
      />
    );
  });

  const { currentDevice } = devicesRes.value;

  return (
    <>
      <TableTitleBar label="Profile" backLink="/settings/profile" />
      <Table>
        <thead>
          <tr>
            <TableHeader showFor="md">ID</TableHeader>
            <th>OS</th>
            <TableHeader showFor="sm">Device</TableHeader>
            <th>Last login</th>
            <th></th>
          </tr>
        </thead>
        <tbody>
          <DeviceTableItem
            key={currentDevice.id.toString()}
            device={currentDevice}
            checkpoint={checkpointsRes.value.checkpoints.find(({ id }) => id === currentDevice.id)}
            onDelete={handleDelete}
            current
          />
          {rows}
        </tbody>
      </Table>
    </>
  );
};

export default DevicesList;
