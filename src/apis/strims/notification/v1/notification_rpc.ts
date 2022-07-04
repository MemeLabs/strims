import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  strims_notification_v1_IWatchRequest,
  strims_notification_v1_WatchRequest,
  strims_notification_v1_WatchResponse,
  strims_notification_v1_IDismissRequest,
  strims_notification_v1_DismissRequest,
  strims_notification_v1_DismissResponse,
} from "./notification";

export interface NotificationFrontendService {
  watch(req: strims_notification_v1_WatchRequest, call: strims_rpc_Call): GenericReadable<strims_notification_v1_WatchResponse>;
  dismiss(req: strims_notification_v1_DismissRequest, call: strims_rpc_Call): Promise<strims_notification_v1_DismissResponse> | strims_notification_v1_DismissResponse;
}

export class UnimplementedNotificationFrontendService implements NotificationFrontendService {
  watch(req: strims_notification_v1_WatchRequest, call: strims_rpc_Call): GenericReadable<strims_notification_v1_WatchResponse> { throw new Error("not implemented"); }
  dismiss(req: strims_notification_v1_DismissRequest, call: strims_rpc_Call): Promise<strims_notification_v1_DismissResponse> | strims_notification_v1_DismissResponse { throw new Error("not implemented"); }
}

export const registerNotificationFrontendService = (host: strims_rpc_Service, service: NotificationFrontendService): void => {
  host.registerMethod<strims_notification_v1_WatchRequest, strims_notification_v1_WatchResponse>("strims.notification.v1.NotificationFrontend.Watch", service.watch.bind(service), strims_notification_v1_WatchRequest);
  host.registerMethod<strims_notification_v1_DismissRequest, strims_notification_v1_DismissResponse>("strims.notification.v1.NotificationFrontend.Dismiss", service.dismiss.bind(service), strims_notification_v1_DismissRequest);
}

export class NotificationFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public watch(req?: strims_notification_v1_IWatchRequest): GenericReadable<strims_notification_v1_WatchResponse> {
    return this.host.expectMany(this.host.call("strims.notification.v1.NotificationFrontend.Watch", new strims_notification_v1_WatchRequest(req)), strims_notification_v1_WatchResponse);
  }

  public dismiss(req?: strims_notification_v1_IDismissRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_notification_v1_DismissResponse> {
    return this.host.expectOne(this.host.call("strims.notification.v1.NotificationFrontend.Dismiss", new strims_notification_v1_DismissRequest(req)), strims_notification_v1_DismissResponse, opts);
  }
}

