import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";
import { Readable as GenericReadable } from "@memelabs/protobuf/lib/rpc/stream";

import {
  IWatchRequest,
  WatchRequest,
  WatchResponse,
  IDismissRequest,
  DismissRequest,
  DismissResponse,
} from "./notification";

export interface NotificationFrontendService {
  watch(req: WatchRequest, call: strims_rpc_Call): GenericReadable<WatchResponse>;
  dismiss(req: DismissRequest, call: strims_rpc_Call): Promise<DismissResponse> | DismissResponse;
}

export class UnimplementedNotificationFrontendService implements NotificationFrontendService {
  watch(req: WatchRequest, call: strims_rpc_Call): GenericReadable<WatchResponse> { throw new Error("not implemented"); }
  dismiss(req: DismissRequest, call: strims_rpc_Call): Promise<DismissResponse> | DismissResponse { throw new Error("not implemented"); }
}

export const registerNotificationFrontendService = (host: strims_rpc_Service, service: NotificationFrontendService): void => {
  host.registerMethod<WatchRequest, WatchResponse>("strims.notification.v1.NotificationFrontend.Watch", service.watch.bind(service), WatchRequest);
  host.registerMethod<DismissRequest, DismissResponse>("strims.notification.v1.NotificationFrontend.Dismiss", service.dismiss.bind(service), DismissRequest);
}

export class NotificationFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public watch(req?: IWatchRequest): GenericReadable<WatchResponse> {
    return this.host.expectMany(this.host.call("strims.notification.v1.NotificationFrontend.Watch", new WatchRequest(req)), WatchResponse);
  }

  public dismiss(req?: IDismissRequest, opts?: strims_rpc_UnaryCallOptions): Promise<DismissResponse> {
    return this.host.expectOne(this.host.call("strims.notification.v1.NotificationFrontend.Dismiss", new DismissRequest(req)), DismissResponse, opts);
  }
}

