import React, { createContext, useCallback, useMemo, useState } from "react";

import { NetworkEvent, Network as networkv1_Network } from "../apis/strims/network/v1/network";
import { useClient } from "./FrontendApi";

type State = Network[];

type Ops = {
  setDisplayOrder: (srcIndex: number, dstIndex: number) => void;
};

const initialState: State = [];

export const NetworkContext = createContext<[State, Ops]>(null);

interface Network {
  network: networkv1_Network;
  peerCount: number;
}

export const Provider: React.FC = ({ children }) => {
  const client = useClient();
  const [items, setItems] = useState<State>(initialState);

  const addItem = (item: Network) =>
    setItems((prev) => {
      const next = Array.from(prev);
      const i = next.findIndex(
        ({ network: { displayOrder } }) => displayOrder > item.network.displayOrder
      );
      next.splice(i === -1 ? next.length : i, 0, item);
      return next;
    });

  const removeItem = (networkId: bigint) =>
    setItems((prev) => prev.filter((item) => item.network.id !== networkId));

  const setPeerCount = (networkId: bigint, peerCount: number) =>
    setItems((prev) =>
      prev.map((item) => (item.network.id === networkId ? { ...item, peerCount: peerCount } : item))
    );

  const setDisplayOrder = useCallback(
    (srcIndex: number, dstIndex: number) =>
      setItems((prev) => {
        const next = Array.from(prev);
        const [target] = next.splice(srcIndex, 1);
        next.splice(dstIndex, 0, target);
        void client.network.setDisplayOrder({ networkIds: next.map(({ network: { id } }) => id) });
        return next;
      }),
    []
  );

  React.useEffect(() => {
    const events = client.network.watch();
    events.on("data", ({ event: { body } }) => {
      switch (body.case) {
        case NetworkEvent.BodyCase.NETWORK_START:
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
      }
    });

    return () => events.destroy();
  }, []);

  const value = useMemo<[State, Ops]>(() => [items, { setDisplayOrder }], [items, setDisplayOrder]);

  return <NetworkContext.Provider value={value}>{children}</NetworkContext.Provider>;
};

Provider.displayName = "Network.Provider";
