// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useTranslation } from "react-i18next";
import { useParams } from "react-router-dom";
import { useTitle } from "react-use";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import NetworkDirectoryForm, { NetworkDirectoryFormData } from "./NetworkDirectoryForm";

const NetworkDirectoryEditForm: React.FC = () => {
  const { t } = useTranslation();
  useTitle(t("settings.network.title"));

  const { networkId } = useParams<"networkId">();
  const [{ value, ...getRes }] = useCall("network", "get", { args: [{ id: BigInt(networkId) }] });

  const network = value?.network;

  const [updateRes, updateServerConfig] = useLazyCall("network", "updateServerConfig");

  if (getRes.loading || !network.serverConfig) {
    return null;
  }

  const { serverConfig } = network;

  const onSubmit = (data: NetworkDirectoryFormData) =>
    updateServerConfig({
      networkId: BigInt(networkId),
      serverConfig: {
        ...serverConfig,
        directory: {
          integrations: {
            angelthump: {
              enable: data.angelthumpEnable,
            },
            twitch: {
              enable: data.twitchEnable,
              clientId: data.twitchClientId,
              clientSecret: data.twitchClientSecret,
            },
            youtube: {
              enable: data.youtubeEnable,
              publicApiKey: data.youtubePublicApiKey,
            },
            swarm: {
              enable: data.swarmEnable,
            },
          },
        },
      },
    });

  const data: NetworkDirectoryFormData = {
    angelthumpEnable: serverConfig.directory?.integrations?.angelthump?.enable,
    twitchEnable: serverConfig.directory?.integrations?.twitch?.enable,
    twitchClientId: serverConfig.directory?.integrations?.twitch?.clientId,
    twitchClientSecret: serverConfig.directory?.integrations?.twitch?.clientSecret,
    youtubeEnable: serverConfig.directory?.integrations?.youtube?.enable,
    youtubePublicApiKey: serverConfig.directory?.integrations?.youtube?.publicApiKey,
    swarmEnable: serverConfig.directory?.integrations?.swarm?.enable,
  };

  return (
    <>
      <TableTitleBar label="Edit Network" backLink={`/settings/networks/${network.id}`} />
      <NetworkDirectoryForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        values={data}
      />
    </>
  );
};

export default NetworkDirectoryEditForm;
