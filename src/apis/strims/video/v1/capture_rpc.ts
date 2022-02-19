import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ICaptureOpenRequest,
  CaptureOpenRequest,
  CaptureOpenResponse,
  ICaptureUpdateRequest,
  CaptureUpdateRequest,
  CaptureUpdateResponse,
  ICaptureAppendRequest,
  CaptureAppendRequest,
  CaptureAppendResponse,
  ICaptureCloseRequest,
  CaptureCloseRequest,
  CaptureCloseResponse,
} from "./capture";

export interface CaptureService {
  open(req: CaptureOpenRequest, call: strims_rpc_Call): Promise<CaptureOpenResponse> | CaptureOpenResponse;
  update(req: CaptureUpdateRequest, call: strims_rpc_Call): Promise<CaptureUpdateResponse> | CaptureUpdateResponse;
  append(req: CaptureAppendRequest, call: strims_rpc_Call): Promise<CaptureAppendResponse> | CaptureAppendResponse;
  close(req: CaptureCloseRequest, call: strims_rpc_Call): Promise<CaptureCloseResponse> | CaptureCloseResponse;
}

export class UnimplementedCaptureService implements CaptureService {
  open(req: CaptureOpenRequest, call: strims_rpc_Call): Promise<CaptureOpenResponse> | CaptureOpenResponse { throw new Error("not implemented"); }
  update(req: CaptureUpdateRequest, call: strims_rpc_Call): Promise<CaptureUpdateResponse> | CaptureUpdateResponse { throw new Error("not implemented"); }
  append(req: CaptureAppendRequest, call: strims_rpc_Call): Promise<CaptureAppendResponse> | CaptureAppendResponse { throw new Error("not implemented"); }
  close(req: CaptureCloseRequest, call: strims_rpc_Call): Promise<CaptureCloseResponse> | CaptureCloseResponse { throw new Error("not implemented"); }
}

export const registerCaptureService = (host: strims_rpc_Service, service: CaptureService): void => {
  host.registerMethod<CaptureOpenRequest, CaptureOpenResponse>("strims.video.v1.Capture.Open", service.open.bind(service), CaptureOpenRequest);
  host.registerMethod<CaptureUpdateRequest, CaptureUpdateResponse>("strims.video.v1.Capture.Update", service.update.bind(service), CaptureUpdateRequest);
  host.registerMethod<CaptureAppendRequest, CaptureAppendResponse>("strims.video.v1.Capture.Append", service.append.bind(service), CaptureAppendRequest);
  host.registerMethod<CaptureCloseRequest, CaptureCloseResponse>("strims.video.v1.Capture.Close", service.close.bind(service), CaptureCloseRequest);
}

export class CaptureClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: ICaptureOpenRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CaptureOpenResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Open", new CaptureOpenRequest(req)), CaptureOpenResponse, opts);
  }

  public update(req?: ICaptureUpdateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CaptureUpdateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Update", new CaptureUpdateRequest(req)), CaptureUpdateResponse, opts);
  }

  public append(req?: ICaptureAppendRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CaptureAppendResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Append", new CaptureAppendRequest(req)), CaptureAppendResponse, opts);
  }

  public close(req?: ICaptureCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<CaptureCloseResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Close", new CaptureCloseRequest(req)), CaptureCloseResponse, opts);
  }
}

