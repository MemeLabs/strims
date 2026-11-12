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

const ratePattern = new RegExp(`^[0-9\\.,]+\\s*(${Object.keys(units).join("|")})$`);

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

const parseUnits = (s: string): bigint => {
  const unit = Object.keys(units).find((unit) => s.endsWith(unit));
  const [n, k] = unit ? [s.substring(0, s.length - unit.length), units[unit]] : [s, BigInt(1)];
  try {
    return BigInt(n.trim()) * k;
  } catch {
    return BigInt(-1);
  }
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
    <form className="thing_form" onSubmit={onSubmit}>
      {error && <InputError error={error.message || "Error creating network"} />}
      <TextInput
        control={control}
        rules={{
          required: {
            value: true,
            message: "Value is required",
          },
          pattern: {
            value: ratePattern,
            message: "Invalid format",
          },
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
          pattern: {
            value: ratePattern,
            message: "Invalid format",
          },
        }}
        label="Max Peers"
        name="maxPeers"
        type="number"
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
