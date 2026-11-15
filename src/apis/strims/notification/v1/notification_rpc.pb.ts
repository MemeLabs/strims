// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";
import type { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  type IWatchRequest as strims_notification_v1_IWatchRequest,
  WatchRequest as strims_notification_v1_WatchRequest,
  WatchResponse as strims_notification_v1_WatchResponse,
  type IDismissRequest as strims_notification_v1_IDismissRequest,
  DismissRequest as strims_notification_v1_DismissRequest,
  DismissResponse as strims_notification_v1_DismissResponse,
} from "./notification.pb";

export interface NotificationFrontendService {
  watch(req: strims_notification_v1_WatchRequest, call: strims_rpc_Call): GenericReadable<strims_notification_v1_WatchResponse>;
  dismiss(req: strims_notification_v1_DismissRequest, call: strims_rpc_Call): Promise<strims_notification_v1_DismissResponse> | strims_notification_v1_DismissResponse;
}

export class UnimplementedNotificationFrontendService implements NotificationFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  watch(req: strims_notification_v1_WatchRequest, call: strims_rpc_Call): GenericReadable<strims_notification_v1_WatchResponse> { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  dismiss(req: strims_notification_v1_DismissRequest, call: strims_rpc_Call): Promise<strims_notification_v1_DismissResponse> | strims_notification_v1_DismissResponse { throw new Error("not implemented"); }
}

export const registerNotificationFrontendService = (host: strims_rpc_Service, service: NotificationFrontendService): void => {
  host.registerMethod<strims_notification_v1_WatchRequest, strims_notification_v1_WatchResponse>("strims.notification.v1.NotificationFrontend.Watch", (req, call) => service.watch(req, call), strims_notification_v1_WatchRequest);
  host.registerMethod<strims_notification_v1_DismissRequest, strims_notification_v1_DismissResponse>("strims.notification.v1.NotificationFrontend.Dismiss", (req, call) => service.dismiss(req, call), strims_notification_v1_DismissRequest);
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

