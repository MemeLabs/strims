// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { createContext, useContext, useEffect, useMemo, useState } from "react";

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
  pushTransientNotification: (notification: INotification) => void;
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

export const Provider: React.FC = ({ children }) => {
  const [state, setState] = useState(initialState);

  const client = useClient();
  useEffect(() => {
    const events = client.notification.watch();
    events.on("data", ({ event }) => setState((prev) => reduceState(prev, event)));
    return () => events.destroy();
  }, []);

  const pushTransientNotification = useStableCallback((notification: INotification) =>
    setState((prev) => ({
      ...prev,
      notifications: [
        ...prev.notifications,
        new Notification({
          createdAt: BigInt(Date.now()),
          status: Notification.Status.STATUS_INFO,
          ...notification,
        }),
      ],
    }))
  );

  const value = useMemo(
    () => ({
      ...state,
      pushTransientNotification,
    }),
    [state]
  );

  return <NotificationContext.Provider value={value}>{children}</NotificationContext.Provider>;
};

Provider.displayName = "Notification.Provider";

export const useNotification = (): NotificationContextValue => useContext(NotificationContext);

export const Consumer = NotificationContext.Consumer;
