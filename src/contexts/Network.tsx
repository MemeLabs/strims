// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, createContext, useCallback, useEffect, useMemo, useState } from "react";

import { AssetBundle } from "../apis/strims/network/v1/directory/directory";
import { Network, NetworkEvent, UIConfig } from "../apis/strims/network/v1/network";
import { useClient } from "./FrontendApi";

interface Value {
  items: Item[];
  assetBundles: Map<bigint, AssetBundle>;
  config: UIConfig;
  updateDisplayOrder: (networkIds: bigint[]) => void;
}

export const NetworkContext = createContext<Value>(null);

interface Item {
  network: Network;
  peerCount: number;
}

interface ProviderProps {
  children: ReactNode;
}

export const Provider: React.FC<ProviderProps> = ({ children }) => {
  const client = useClient();
  const [items, setItems] = useState<Item[]>([]);
  const [assetBundles, setAssetBundles] = useState<Map<bigint, AssetBundle>>(new Map());
  const [config, setConfig] = useState<UIConfig>(null);

  const addItem = (item: Item) => setItems((prev) => [...prev, item]);

  const removeItem = (networkId: bigint) =>
    setItems((prev) => prev.filter((item) => item.network.id !== networkId));

  const setPeerCount = (networkId: bigint, peerCount: number) =>
    setItems((prev) =>
      prev.map((item) => (item.network.id === networkId ? { ...item, peerCount } : item))
    );

  const setAssetBundle = (networkId: bigint, assetBundle: AssetBundle) =>
    setAssetBundles((prev) => new Map(prev).set(networkId, assetBundle));

  const removeAssetBundle = (networkId: bigint) =>
    setAssetBundles((prev) => {
      const bundles = new Map(prev);
      bundles.delete(networkId);
      return bundles;
    });

  const updateDisplayOrder = useCallback(
    (networkIds: bigint[]) =>
      setConfig((prev) => {
        void client.network.updateDisplayOrder({ networkIds });
        return new UIConfig({
          ...prev,
          networkDisplayOrder: networkIds,
        });
      }),
    []
  );

  useEffect(() => {
    void client.network.getUIConfig().then(({ config }) => setConfig(config));

    const events = client.network.watch();
    events.on("data", ({ event: { body } }) => {
      switch (body.case) {
        case NetworkEvent.BodyCase.NETWORK_START:
          removeItem(body.networkStart.network.id);
          addItem(body.networkStart);
          break;
        case NetworkEvent.BodyCase.NETWORK_STOP:
          removeItem(body.networkStop.networkId);
          removeAssetBundle(body.networkStop.networkId);
          break;
        case NetworkEvent.BodyCase.NETWORK_PEER_COUNT_UPDATE:
          setPeerCount(
            body.networkPeerCountUpdate.networkId,
            body.networkPeerCountUpdate.peerCount
          );
          break;
        case NetworkEvent.BodyCase.UI_CONFIG_UPDATE:
          setConfig(body.uiConfigUpdate);
      }
    });
    return () => events.destroy();
  }, []);

  useEffect(() => {
    const events = client.directory.watchAssetBundles();
    events.on("data", ({ networkId, assetBundle }) => {
      setAssetBundle(networkId, assetBundle);
    });
    return () => events.destroy();
  }, []);

  const value = useMemo<Value>(
    () => ({ items, assetBundles, config, updateDisplayOrder }),
    [items, assetBundles, config, updateDisplayOrder]
  );

  return <NetworkContext.Provider value={value}>{children}</NetworkContext.Provider>;
};

Provider.displayName = "Network.Provider";
