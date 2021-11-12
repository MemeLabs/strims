import React, { createContext, useContext, useEffect, useReducer } from "react";

import { Event, Notification } from "../apis/strims/notification/v1/notification";
import { useClient } from "./FrontendApi";

export interface State {
  notifications: Notification[];
  count: number;
}

const initialState: State = {
  notifications: [],
  count: 0,
};

const NotificationContext = createContext<State>(null);

const reduceState = (prev: State, { body }: Event): State => {
  console.log(">>>", body);
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
  const [state, dispatch] = useReducer(reduceState, initialState);

  const client = useClient();
  useEffect(() => {
    const events = client.notification.watch();
    events.on("data", ({ event }) => dispatch(event));
    return () => events.destroy();
  }, []);

  return <NotificationContext.Provider value={state}>{children}</NotificationContext.Provider>;
};

Provider.displayName = "Notification.Provider";

export const useNotification = (): State => useContext(NotificationContext);

export const Consumer = NotificationContext.Consumer;
