// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { useCallback, useContext, useMemo } from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useInterval, useTitle } from "react-use";

import { MockStream } from "../../../apis/strims/debug/v1/metrics";
import {
  Button,
  ButtonSet,
  InputError,
  NetworkSelectInput,
  SelectInput,
  SelectOption,
  TextInput,
} from "../../../components/Form";
import {
  MenuCell,
  MenuItem,
  Table,
  TableCell,
  TableTitleBar,
} from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import { NetworkContext } from "../../../contexts/Network";
import { certificateRoot } from "../../../lib/certificate";

const timeoutMsOptions = [
  {
    value: 60000,
    label: "1 minute",
  },
  {
    value: 300000,
    label: "5 minutes",
  },
  {
    value: 600000,
    label: "10 minutes",
  },
  {
    value: 1800000,
    label: "30 minutes",
  },
  {
    value: 3600000,
    label: "1 hour",
  },
];

const formatDuration = (ms: number) => {
  if (ms === 0) {
    return "none";
  }
  if (ms % 3600000 === 0) {
    return `${ms / 3600000} h`;
  }
  if (ms % 60000 === 0) {
    return `${ms / 60000} min`;
  }
  return `${ms / 1000} s`;
};

interface MockStreamTableItemProps {
  mockStream: MockStream;
  networkName: string;
  onStop: () => void;
}

const MockStreamTableItem = ({ mockStream, networkName, onStop }: MockStreamTableItemProps) => {
  const [, stopMockStream] = useLazyCall("debug", "stopMockStream", { onComplete: onStop });

  const handleStop = useCallback(() => stopMockStream({ id: mockStream.id }), [mockStream]);

  const startedAt = new Date(Number(mockStream.startedAt / BigInt(1000000)));

  return (
    <tr>
      <TableCell>{mockStream.id.toString()}</TableCell>
      <TableCell truncate>{networkName}</TableCell>
      <TableCell>{mockStream.bitrateKbps} kbps</TableCell>
      <TableCell>{mockStream.segmentIntervalMs} ms</TableCell>
      <TableCell title={startedAt.toLocaleString()}>{startedAt.toLocaleTimeString()}</TableCell>
      <TableCell>{formatDuration(mockStream.timeoutMs)}</TableCell>
      <TableCell>{mockStream.segments.toString()}</TableCell>
      <TableCell title={mockStream.listingId?.value.toString()}>
        {mockStream.listingId ? "yes" : "no"}
      </TableCell>
      <MenuCell>
        <MenuItem label="Stop" onClick={handleStop} />
      </MenuCell>
    </tr>
  );
};

interface DebugMockStreamData {
  bitrateKbps: number;
  segmentIntervalMs: number;
  timeoutMs: SelectOption<number>;
  networkKey: string;
}

const DebugMockStream = () => {
  const { t } = useTranslation();
  useTitle(t("settings.debug.title"));

  const [mockStreamsRes, listMockStreams] = useCall("debug", "listMockStreams");
  useInterval(() => void listMockStreams(), 2000);

  const [startMockStreamRes, startMockStream] = useLazyCall("debug", "startMockStream", {
    onComplete: () => void listMockStreams(),
  });

  // Network names by base64 key.
  const networks = useContext(NetworkContext);
  const networkNames = useMemo(
    () =>
      new Map(
        networks?.items.map(({ network }): [string, string] => {
          const { key, subject } = certificateRoot(network.certificate);
          return [Base64.fromUint8Array(key), subject];
        })
      ),
    [networks?.items]
  );

  const { handleSubmit, control, formState } = useForm<DebugMockStreamData>({
    mode: "onBlur",
    defaultValues: {
      bitrateKbps: 6000,
      segmentIntervalMs: 1000,
      timeoutMs: timeoutMsOptions[0],
    },
  });

  const onSubmit = handleSubmit(async (data) => {
    return startMockStream({
      bitrateKbps: data.bitrateKbps,
      segmentIntervalMs: data.segmentIntervalMs,
      timeoutMs: data.timeoutMs.value,
      networkKey: Base64.toUint8Array(data.networkKey),
    });
  });

  const rows = mockStreamsRes.value?.mockStreams.map((mockStream) => {
    const key = Base64.fromUint8Array(mockStream.networkKey);
    return (
      <MockStreamTableItem
        key={mockStream.id.toString()}
        mockStream={mockStream}
        networkName={networkNames.get(key) ?? key.substring(0, 8)}
        onStop={() => void listMockStreams()}
      />
    );
  });

  return (
    <>
      <TableTitleBar label="Mock stream" backLink="/settings/debug" />
      <form className="thing_form" onSubmit={onSubmit}>
        {startMockStreamRes.error && (
          <InputError error={startMockStreamRes.error.message || "Error creating mock stream"} />
        )}
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Bitrate is required",
            },
            min: {
              value: 0,
              message: "Bitrate must be at least 0 kbps",
            },
            max: {
              value: 50000,
              message: "Bitrate must be at most 50000 kbps",
            },
          }}
          name="bitrateKbps"
          label="Bitrate (kbps)"
          type="number"
        />
        <TextInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Segment interval is required",
            },
            min: {
              value: 100,
              message: "Segment interval must be at least 100 ms",
            },
            max: {
              value: 10000,
              message: "Segment interval must be at most 10000 ms",
            },
          }}
          name="segmentIntervalMs"
          label="Segment interval (ms)"
          type="number"
        />
        <SelectInput
          control={control}
          label="Duration"
          name="timeoutMs"
          options={timeoutMsOptions}
        />
        <NetworkSelectInput
          control={control}
          rules={{
            required: {
              value: true,
              message: "Network is required",
            },
          }}
          name="networkKey"
          label="Network"
          placeholder="Select network"
        />
        <ButtonSet>
          <Button disabled={formState.isSubmitting}>Start mock stream</Button>
        </ButtonSet>
      </form>
      {!!rows?.length && (
        <Table>
          <thead>
            <tr>
              <th>ID</th>
              <th>Network</th>
              <th>Bitrate</th>
              <th>Interval</th>
              <th>Started</th>
              <th>Timeout</th>
              <th>Segments</th>
              <th>Listed</th>
              <th></th>
            </tr>
          </thead>
          <tbody>{rows}</tbody>
        </Table>
      )}
    </>
  );
};

export default DebugMockStream;
