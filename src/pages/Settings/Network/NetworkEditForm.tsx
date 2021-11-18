import { Base64 } from "js-base64";
import React from "react";
import { useParams } from "react-router-dom";

import { INetwork, Network } from "../../../apis/strims/network/v1/network";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import NetworkForm, { NetworkFormData } from "./NetworkForm";

const NetworkEditForm: React.FC = () => {
  const { networkId } = useParams<"networkId">();
  const [{ value, ...getRes }] = useCall("network", "get", { args: [{ id: BigInt(networkId) }] });

  const network = value?.network;

  const [updateRes, updateServerConfig] = useLazyCall("network", "updateServerConfig");

  if (getRes.loading) {
    return null;
  }
  if (network?.serverConfigOneof?.case !== Network.ServerConfigOneofCase.SERVER_CONFIG) {
    // TODO: error message
    return null;
  }

  const { serverConfig } = network?.serverConfigOneof;

  const onSubmit = (data: NetworkFormData) =>
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

  const data: NetworkFormData = {
    angelthumpEnable: serverConfig.directory?.integrations?.angelthump?.enable,
    twitchEnable: serverConfig.directory?.integrations?.twitch?.enable,
    twitchClientId: serverConfig.directory?.integrations?.twitch?.clientId,
    twitchClientSecret: serverConfig.directory?.integrations?.twitch?.clientSecret,
    youtubeEnable: serverConfig.directory?.integrations?.youtube?.enable,
    youtubePublicApiKey: serverConfig.directory?.integrations?.youtube?.publicApiKey,
    swarmEnable: serverConfig.directory?.integrations?.swarm?.enable,
  };

  return (
    <NetworkForm
      onSubmit={onSubmit}
      error={getRes.error || updateRes.error}
      loading={getRes.loading || updateRes.loading}
      values={data}
      indexLinkVisible={true}
    />
  );
};

export default NetworkEditForm;
