// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { ReactNode, useCallback, useMemo, useState } from "react";

import { AssetBundle } from "../../apis/strims/network/v1/directory/directory";
import { Network, UIConfig } from "../../apis/strims/network/v1/network";
import { NetworkContext } from "../../contexts/Network";
import { Provider as NotificationProvider } from "../../contexts/Notification";
import { PlayerContext, PlayerMode } from "../../contexts/Player";
import NotificationService from "../mocks/notification/service";
import { networkItems } from "./fixtures/networks";
import { MockApi, useMockServices } from "./MockApi";

export { SessionProvider } from "../contexts/Session";

type PlayerValue = React.ContextType<typeof PlayerContext>;

const noop = (): void => undefined;

const defaultPlayer: PlayerValue = {
  path: "",
  source: null,
  mode: PlayerMode.PIP,
  networkKey: null,
  listingId: null,
  setPath: noop,
  setSource: noop,
  setMode: noop,
};

export interface StaticPlayerProps {
  /** Fields over the defaults: nothing playing, PIP mode, no-op setters. */
  value?: Partial<PlayerValue>;
  children: ReactNode;
}

// PlayerContext without the real provider's directory calls.
export const StaticPlayer: React.FC<StaticPlayerProps> = ({ value, children }) => {
  const v = useMemo(() => ({ ...defaultPlayer, ...value }), [value]);
  return <PlayerContext.Provider value={v}>{children}</PlayerContext.Provider>;
};

type NetworkItem = { network: Network; peerCount: number };

export interface StaticNetworkProps {
  /** Default: fixture networks 1..5. */
  items?: NetworkItem[];
  /** Default: items in their order. */
  config?: UIConfig;
  assetBundles?: Map<bigint, AssetBundle>;
  children: ReactNode;
}

// NetworkContext from props, without the real provider's watch streams;
// updateDisplayOrder reorders it.
export const StaticNetwork: React.FC<StaticNetworkProps> = ({
  items: itemsProp,
  config: configProp,
  assetBundles: assetBundlesProp,
  children,
}) => {
  const items = useMemo(() => itemsProp ?? networkItems(5), [itemsProp]);
  const assetBundles = useMemo(() => assetBundlesProp ?? new Map(), [assetBundlesProp]);
  const [order, setOrder] = useState<bigint[]>(null);
  const config = useMemo(
    () =>
      order
        ? new UIConfig({ networkDisplayOrder: order })
        : configProp ?? new UIConfig({ networkDisplayOrder: items.map(({ network }) => network.id) }),
    [order, configProp, items]
  );
  const updateDisplayOrder = useCallback((ids: bigint[]) => setOrder(ids), []);
  const value = useMemo(
    () => ({ items, assetBundles, config, updateDisplayOrder }),
    [items, assetBundles, config]
  );
  return <NetworkContext.Provider value={value}>{children}</NetworkContext.Provider>;
};

export interface MockNotificationsProps {
  /** The mock to watch; default the nearest MockApi's notification, else a new one. */
  service?: NotificationService;
  children: ReactNode;
}

// The real Notification provider over a mock NotificationService. Under a
// MockApi with a notification service it watches that one; otherwise it adds
// `service` (or a new one) to the nearest outer MockApi's services in a
// MockApi of its own, so it goes inside or outside a story's MockApi alike.
// service.emitEvent() pushes notifications.
export const MockNotifications: React.FC<MockNotificationsProps> = ({ service, children }) => {
  const parent = useMockServices();
  if (!service && parent.notification) {
    return <NotificationProvider>{children}</NotificationProvider>;
  }
  return (
    <MockApi services={() => ({ notification: service ?? new NotificationService() })}>
      <NotificationProvider>{children}</NotificationProvider>
    </MockApi>
  );
};
