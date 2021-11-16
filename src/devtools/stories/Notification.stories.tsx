import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../apis/client";
import { Event, Notification } from "../../apis/strims/notification/v1/notification";
import { registerNotificationFrontendService } from "../../apis/strims/notification/v1/notification_rpc";
import Toast from "../../components/Notification/Toast";
import { Provider as ApiProvider } from "../../contexts/FrontendApi";
import {
  Consumer as NotificationConsumer,
  Provider as NotificationProvider,
} from "../../contexts/Notification";
import jsonutil from "../../lib/jsonutil";
import NotificationService from "../mocks/notification/service";

let nextId = BigInt(0);

const Test: React.FC = () => {
  const [[service, client]] = React.useState((): [NotificationService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new NotificationService();
    registerNotificationFrontendService(svc, service);

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

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
      <ApiProvider value={client}>
        <NotificationProvider>
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
        </NotificationProvider>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "Notification",
    component: () => <Test />,
  },
];
