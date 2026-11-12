// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, {
  ReactNode,
  createContext,
  useContext,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import { Event, INotification, Notification } from "../apis/strims/notification/v1/notification";
import { useStableCallback } from "../hooks/useStableCallback";
import { useClient } from "./FrontendApi";

export interface State {
  notifications: Notification[];
  count: number;
}

const initialState: State = {
  notifications: [],
  count: 0,
};

export interface NotificationContextValue extends State {
  /** Shows a notification of the page's own, which the node never hears of. */
  pushTransientNotification: (notification: INotification) => void;
  /** Dismisses notification `id`: the page's own here, the node's on the node. */
  dismissNotification: (id: bigint) => void;
}

const NotificationContext = createContext<NotificationContextValue>(null);

const reduceState = (prev: State, { body }: Event): State => {
  switch (body.case) {
    case Event.BodyCase.NOTIFICATION:
      return {
        ...prev,
        notifications: [...prev.notifications, body.notification],
      };
    case Event.BodyCase.DISMISS:
      return {
        ...prev,
        notifications: prev.notifications.filter(({ id }) => id !== body.dismiss),
      };
  }
};

interface ProviderProps {
  children: ReactNode;
}

export const Provider: React.FC<ProviderProps> = ({ children }) => {
  const [state, setState] = useState(initialState);

  const client = useClient();
  useEffect(() => {
    const events = client.notification.watch();
    events.on("data", ({ event }) => setState((prev) => reduceState(prev, event)));
    return () => events.destroy();
  }, []);

  // The page's own notifications take negative ids, which no node
  // notification (its id a uint64) has, so they dismiss here.
  const lastTransientId = useRef(BigInt(0));
  const pushTransientNotification = useStableCallback((notification: INotification) => {
    lastTransientId.current -= BigInt(1);
    const id = lastTransientId.current;
    setState((prev) => ({
      ...prev,
      notifications: [
        ...prev.notifications,
        new Notification({
          createdAt: BigInt(Date.now()),
          status: Notification.Status.STATUS_INFO,
          ...notification,
          id,
        }),
      ],
    }));
  });

  // A node notification goes when the node says it's dismissed; a failed
  // dismiss leaves its toast to time out.
  const dismissNotification = useStableCallback((id: bigint) => {
    if (id < BigInt(0)) {
      setState((prev) => ({
        ...prev,
        notifications: prev.notifications.filter((n) => n.id !== id),
      }));
    } else {
      void client.notification.dismiss({ ids: [id] }).catch(() => undefined);
    }
  });

  const value = useMemo(
    () => ({
      ...state,
      pushTransientNotification,
      dismissNotification,
    }),
    [state]
  );

  return <NotificationContext.Provider value={value}>{children}</NotificationContext.Provider>;
};

Provider.displayName = "Notification.Provider";

export const useNotification = (): NotificationContextValue => useContext(NotificationContext);

export const Consumer = NotificationContext.Consumer;
