import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  strims_video_v1_ICaptureOpenRequest,
  strims_video_v1_CaptureOpenRequest,
  strims_video_v1_CaptureOpenResponse,
  strims_video_v1_ICaptureUpdateRequest,
  strims_video_v1_CaptureUpdateRequest,
  strims_video_v1_CaptureUpdateResponse,
  strims_video_v1_ICaptureAppendRequest,
  strims_video_v1_CaptureAppendRequest,
  strims_video_v1_CaptureAppendResponse,
  strims_video_v1_ICaptureCloseRequest,
  strims_video_v1_CaptureCloseRequest,
  strims_video_v1_CaptureCloseResponse,
} from "./capture";

export interface CaptureService {
  open(req: strims_video_v1_CaptureOpenRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureOpenResponse> | strims_video_v1_CaptureOpenResponse;
  update(req: strims_video_v1_CaptureUpdateRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureUpdateResponse> | strims_video_v1_CaptureUpdateResponse;
  append(req: strims_video_v1_CaptureAppendRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureAppendResponse> | strims_video_v1_CaptureAppendResponse;
  close(req: strims_video_v1_CaptureCloseRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureCloseResponse> | strims_video_v1_CaptureCloseResponse;
}

export class UnimplementedCaptureService implements CaptureService {
  open(req: strims_video_v1_CaptureOpenRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureOpenResponse> | strims_video_v1_CaptureOpenResponse { throw new Error("not implemented"); }
  update(req: strims_video_v1_CaptureUpdateRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureUpdateResponse> | strims_video_v1_CaptureUpdateResponse { throw new Error("not implemented"); }
  append(req: strims_video_v1_CaptureAppendRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureAppendResponse> | strims_video_v1_CaptureAppendResponse { throw new Error("not implemented"); }
  close(req: strims_video_v1_CaptureCloseRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureCloseResponse> | strims_video_v1_CaptureCloseResponse { throw new Error("not implemented"); }
}

export const registerCaptureService = (host: strims_rpc_Service, service: CaptureService): void => {
  host.registerMethod<strims_video_v1_CaptureOpenRequest, strims_video_v1_CaptureOpenResponse>("strims.video.v1.Capture.Open", service.open.bind(service), strims_video_v1_CaptureOpenRequest);
  host.registerMethod<strims_video_v1_CaptureUpdateRequest, strims_video_v1_CaptureUpdateResponse>("strims.video.v1.Capture.Update", service.update.bind(service), strims_video_v1_CaptureUpdateRequest);
  host.registerMethod<strims_video_v1_CaptureAppendRequest, strims_video_v1_CaptureAppendResponse>("strims.video.v1.Capture.Append", service.append.bind(service), strims_video_v1_CaptureAppendRequest);
  host.registerMethod<strims_video_v1_CaptureCloseRequest, strims_video_v1_CaptureCloseResponse>("strims.video.v1.Capture.Close", service.close.bind(service), strims_video_v1_CaptureCloseRequest);
}

export class CaptureClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public open(req?: strims_video_v1_ICaptureOpenRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_CaptureOpenResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Open", new strims_video_v1_CaptureOpenRequest(req)), strims_video_v1_CaptureOpenResponse, opts);
  }

  public update(req?: strims_video_v1_ICaptureUpdateRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_CaptureUpdateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Update", new strims_video_v1_CaptureUpdateRequest(req)), strims_video_v1_CaptureUpdateResponse, opts);
  }

  public append(req?: strims_video_v1_ICaptureAppendRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_CaptureAppendResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Append", new strims_video_v1_CaptureAppendRequest(req)), strims_video_v1_CaptureAppendResponse, opts);
  }

  public close(req?: strims_video_v1_ICaptureCloseRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_video_v1_CaptureCloseResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Close", new strims_video_v1_CaptureCloseRequest(req)), strims_video_v1_CaptureCloseResponse, opts);
  }
}

