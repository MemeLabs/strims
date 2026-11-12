// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect } from "react";

import {
  Event,
  INotification,
  Notification,
} from "../../../../apis/strims/notification/v1/notification";
import Toast from "../../../../components/Notification/Toast";
import NotificationService from "../../../mocks/notification/service";
import { MockNotifications, Story, useMockService } from "../../../storybook";

const { Status } = Notification;

// Long enough that the toasts are still up when the story is looked at.
const TIMEOUT_MS = 10 * 60 * 1000;

// Pushes the notifications through the mock service once mounted, as the
// node's would arrive.
const Emit: React.FC<{ notifications: INotification[] }> = ({ notifications }) => {
  const service = useMockService<NotificationService>("notification");
  useEffect(() => {
    const createdAt = BigInt(Date.now());
    notifications.forEach((n, i) =>
      service.emitEvent(
        new Event({ body: { notification: { id: BigInt(i + 1), createdAt, ...n } } })
      )
    );
  }, []);
  return null;
};

const Toasts: React.FC<{ notifications: INotification[] }> = ({ notifications }) => (
  <MockNotifications>
    <Emit notifications={notifications} />
    <Toast timeoutMs={TIMEOUT_MS} />
  </MockNotifications>
);

const info: INotification = { status: Status.STATUS_INFO, title: "Network joined" };
const success: INotification = { status: Status.STATUS_SUCCESS, title: "Profile saved" };
const warning: INotification = {
  status: Status.STATUS_WARNING,
  title: "Certificate expires soon",
  message: "Test Network's certificate renews in 2 days.",
};
const error: INotification = {
  status: Status.STATUS_ERROR,
  title: "Stream failed",
  message: "rpc error: no peers answered",
};

const stories: Story[] = [
  {
    name: "Info",
    component: () => <Toasts notifications={[info]} />,
  },
  {
    name: "Success",
    component: () => <Toasts notifications={[success]} />,
  },
  {
    name: "Warning",
    component: () => <Toasts notifications={[warning]} />,
  },
  {
    name: "Error",
    component: () => <Toasts notifications={[error]} />,
  },
  {
    name: "Stacked",
    component: () => <Toasts notifications={[info, success, warning, error]} />,
  },
  {
    name: "Long message",
    component: () => (
      <Toasts
        notifications={[
          {
            status: Status.STATUS_ERROR,
            title: "Couldn't renew the certificate for Test Network",
            message:
              "rpc error: the certificate authority refused the renewal because the alias " +
              "test_user is held by another key in this network; join again under another name " +
              "or ask the network's owner to release the reservation. " +
              "averyveryverylongwordwithnospacesthatcannotwrap",
          },
        ]}
      />
    ),
  },
];

export default stories;
