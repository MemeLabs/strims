import React from "react";
import { useParams } from "react-router-dom";

import { TableTitleBar } from "../../../components/Settings/Table";
import { useCall, useLazyCall } from "../../../contexts/FrontendApi";
import NetworkForm, { NetworkFormData } from "./NetworkForm";

const NetworkEditForm: React.FC = () => {
  const { networkId } = useParams<"networkId">();
  const [{ value, ...getRes }] = useCall("network", "get", { args: [{ id: BigInt(networkId) }] });

  const network = value?.network;

  const [updateRes, updateAlias] = useLazyCall("network", "updateAlias");

  if (getRes.loading) {
    return null;
  }

  const onSubmit = (data: NetworkFormData) =>
    updateAlias({
      id: network.id,
      alias: data.alias,
    });

  const data: NetworkFormData = {
    alias: network.alias,
  };

  return (
    <>
      <TableTitleBar label="Edit Network" backLink="/settings/networks" />
      <NetworkForm
        onSubmit={onSubmit}
        error={getRes.error || updateRes.error}
        loading={getRes.loading || updateRes.loading}
        networkId={network.id}
        showDirectoryFormLink={!!network.serverConfig}
        values={data}
      />
    </>
  );
};

export default NetworkEditForm;
