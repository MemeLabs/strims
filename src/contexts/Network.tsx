import React, { createContext, useCallback, useMemo, useState } from "react";

import { Network, NetworkEvent, UIConfig } from "../apis/strims/network/v1/network";
import { useClient } from "./FrontendApi";

interface Value {
  items: Item[];
  config: UIConfig;
  updateDisplayOrder: (networkIds: bigint[]) => void;
}

export const NetworkContext = createContext<Value>(null);

interface Item {
  network: Network;
  peerCount: number;
}

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [items, setItems] = useState<Item[]>([]);
  const [config, setConfig] = useState<UIConfig>(null);

  const addItem = (item: Item) => setItems((prev) => [...prev, item]);

  const removeItem = (networkId: bigint) =>
    setItems((prev) => prev.filter((item) => item.network.id !== networkId));

  const setPeerCount = (networkId: bigint, peerCount: number) =>
    setItems((prev) =>
      prev.map((item) => (item.network.id === networkId ? { ...item, peerCount: peerCount } : item))
    );

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

  React.useEffect(() => {
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

  const value = useMemo<Value>(
    () => ({ items: items, config, updateDisplayOrder }),
    [items, config, updateDisplayOrder]
  );

  return <NetworkContext.Provider value={value}>{children}</NetworkContext.Provider>;
};

Provider.displayName = "Network.Provider";
