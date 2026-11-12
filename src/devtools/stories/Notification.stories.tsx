// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import { Event, Notification } from "../../apis/strims/notification/v1/notification";
import Toast from "../../components/Notification/Toast";
import { Consumer as NotificationConsumer } from "../../contexts/Notification";
import jsonutil from "../../lib/jsonutil";
import NotificationService from "../mocks/notification/service";
import { MockNotifications, Story } from "../storybook";

let nextId = BigInt(0);

const Test: React.FC = () => {
  const [service] = React.useState(() => new NotificationService());

  const handleCreateClick = () =>
    service.emitEvent(
      new Event({
        body: {
          notification: {
            id: ++nextId,
            createdAt: BigInt(Date.now()),
            status: Notification.Status.STATUS_INFO,
            title: "test notification",
            message: "",
            subject: {
              model: Notification.Subject.Model.NOTIFICATION_SUBJECT_MODEL_NETWORK,
              id: nextId,
            },
          },
        },
      })
    );

  const handleDismissClick = (id: bigint) =>
    service.emitEvent(
      new Event({
        body: { dismiss: id },
      })
    );

  return (
    <div className="notification_mockup">
      <MockNotifications service={service}>
        <NotificationConsumer>
          {({ notifications }) => (
            <>
              <button className="input input_button" onClick={handleCreateClick}>
                create notif
              </button>
              {!!notifications.length && (
                <button
                  className="input input_button"
                  onClick={() => handleDismissClick(notifications[0].id)}
                >
                  dismiss notif
                </button>
              )}
              <Toast />
              <pre>{jsonutil.stringify(notifications)}</pre>
            </>
          )}
        </NotificationConsumer>
      </MockNotifications>
    </div>
  );
};

const stories: Story[] = [
  {
    name: "Notification",
    component: () => <Test />,
  },
];

export default stories;
