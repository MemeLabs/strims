// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useForm } from "react-hook-form";
import { useTranslation } from "react-i18next";
import { useTitle } from "react-use";

import { Config, SetConfigResponse } from "../../apis/strims/vnic/v1/vnic";
import { Button, ButtonSet, InputError, TextInput } from "../../components/Form";
import { TableTitleBar } from "../../components/Settings/Table";
import { useCall, useLazyCall } from "../../contexts/FrontendApi";

const units: { [key: string]: bigint } = {
  "KiBps": BigInt(Math.pow(2, 10)),
  "MiBps": BigInt(Math.pow(2, 20)),
  "GiBps": BigInt(Math.pow(2, 30)),
  "TiBps": BigInt(Math.pow(2, 40)),
  "KBps": BigInt(1e3),
  "MBps": BigInt(1e6),
  "GBps": BigInt(1e9),
  "TBps": BigInt(1e12),
  "Kibps": BigInt(Math.pow(2, 10) / 8),
  "Mibps": BigInt(Math.pow(2, 20) / 8),
  "Gibps": BigInt(Math.pow(2, 30) / 8),
  "Tibps": BigInt(Math.pow(2, 40) / 8),
  "Kbps": BigInt(1e3 / 8),
  "Mbps": BigInt(1e6 / 8),
  "Gbps": BigInt(1e9 / 8),
  "Tbps": BigInt(1e12 / 8),
  "Bps": BigInt(1),
};
["Kibps", "Mibps", "Gibps", "Tibps", "Kbps", "Mbps", "Gbps", "Tbps"].forEach(
  (unit) => (units[unit.toLowerCase()] = units[unit])
);

// A rate as the form takes it: a number, its thousands grouped with commas or
// not and a decimal fraction allowed, and a unit, e.g. "50mbps",
// "1.5 MiBps" or "1,000Kbps".
const ratePattern = new RegExp(
  `^(\\d{1,3}(?:,\\d{3})+|\\d+)(?:\\.(\\d+))?\\s*(${Object.keys(units).join("|")})$`
);

// The largest rate the config holds (a uint64 of bytes per second).
const MAX_RATE = BigInt("18446744073709551615");

// The largest peer budget the config holds (a uint32).
const MAX_PEERS = 2 ** 32 - 1;

const formatUnits = (n: bigint) => {
  let minSize = Infinity;
  let minS = "";
  Object.entries(units).forEach(([unit, value]) => {
    const s = `${n / value}${unit}`;
    if (n % value == BigInt(0) && s.length < minSize) {
      minSize = s.length;
      minS = s;
    }
  });
  return minS;
};

// The bytes per second a rate says, to the nearest byte (the config holds
// whole bytes), exact for any decimal fraction, and at least one byte for a
// rate above zero (0 is no limit); null when it isn't a rate.
const parseUnits = (s: string): bigint | null => {
  const m = ratePattern.exec(s.trim());
  if (!m) {
    return null;
  }
  const [, whole, fraction = "", unit] = m;
  const scale = BigInt(`1${"0".repeat(fraction.length)}`);
  const scaled = BigInt(whole.replace(/,/g, "") + fraction) * units[unit];
  const n = (scaled + scale / BigInt(2)) / scale;
  return n === BigInt(0) && scaled > BigInt(0) ? BigInt(1) : n;
};

// The form's rule for a rate, the parser's verdict.
const validateRate = (v: string) => {
  const n = parseUnits(v);
  if (n === null) {
    return "Enter a number and a unit, e.g. 50mbps or 1.5 MiBps";
  }
  return n <= MAX_RATE || "Too large";
};

interface VNICFormProps {
  config: Config;
  onCreate: (res: SetConfigResponse) => void;
}

const VNICForm = ({ onCreate, config }: VNICFormProps) => {
  const [{ error, loading }, setConfig] = useLazyCall("vnic", "setConfig", {
    onComplete: onCreate,
  });
  const { control, handleSubmit } = useForm<{
    maxUploadBytesPerSecond: string;
    maxPeers: number;
  }>({
    mode: "onBlur",
    defaultValues: {
      maxUploadBytesPerSecond: formatUnits(config.maxUploadBytesPerSecond),
      maxPeers: config.maxPeers,
    },
  });

  const onSubmit = handleSubmit(async (data) => {
    try {
      await setConfig({
        config: {
          maxUploadBytesPerSecond: parseUnits(data.maxUploadBytesPerSecond),
          maxPeers: data.maxPeers,
        },
      });
    } catch {
      // The form shows error.
    }
  });

  return (
    <form className="thing_form" onSubmit={onSubmit} noValidate>
      {error && <InputError error={error.message || "Error storing the config"} />}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Value is required",
          },
          validate: validateRate,
        }}
        label="Max Upload Rate"
        name="maxUploadBytesPerSecond"
        placeholder="ex. 50mbps"
      />
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Value is required",
          },
          validate: (v: number) =>
            (Number.isInteger(v) && v >= 0 && v <= MAX_PEERS) ||
            `Enter a whole number from 0 to ${MAX_PEERS}`,
        }}
        label="Max Peers"
        description="0 for no limit"
        name="maxPeers"
        type="number"
        min={0}
        step={1}
        placeholder="ex. 25"
      />
      <ButtonSet>
        <Button disabled={loading}>Store Config</Button>
      </ButtonSet>
    </form>
  );
};

const VNICsPage: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.vnic.title"));

  const [{ value, error }, getConfig] = useCall("vnic", "getConfig");

  return (
    <>
      <TableTitleBar label="VNIC" />
      {error && (
        <div className="thing_form">
          <InputError error={error.message || "Error loading config"} />
        </div>
      )}
      {value && <VNICForm config={value.config} onCreate={() => getConfig()} />}
    </>
  );
};

export default VNICsPage;
