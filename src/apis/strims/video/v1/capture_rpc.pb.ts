// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type ICaptureOpenRequest as strims_video_v1_ICaptureOpenRequest,
  CaptureOpenRequest as strims_video_v1_CaptureOpenRequest,
  CaptureOpenResponse as strims_video_v1_CaptureOpenResponse,
  type ICaptureUpdateRequest as strims_video_v1_ICaptureUpdateRequest,
  CaptureUpdateRequest as strims_video_v1_CaptureUpdateRequest,
  CaptureUpdateResponse as strims_video_v1_CaptureUpdateResponse,
  type ICaptureAppendRequest as strims_video_v1_ICaptureAppendRequest,
  CaptureAppendRequest as strims_video_v1_CaptureAppendRequest,
  CaptureAppendResponse as strims_video_v1_CaptureAppendResponse,
  type ICaptureCloseRequest as strims_video_v1_ICaptureCloseRequest,
  CaptureCloseRequest as strims_video_v1_CaptureCloseRequest,
  CaptureCloseResponse as strims_video_v1_CaptureCloseResponse,
} from "./capture.pb";

export interface CaptureService {
  open(req: strims_video_v1_CaptureOpenRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureOpenResponse> | strims_video_v1_CaptureOpenResponse;
  update(req: strims_video_v1_CaptureUpdateRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureUpdateResponse> | strims_video_v1_CaptureUpdateResponse;
  append(req: strims_video_v1_CaptureAppendRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureAppendResponse> | strims_video_v1_CaptureAppendResponse;
  close(req: strims_video_v1_CaptureCloseRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureCloseResponse> | strims_video_v1_CaptureCloseResponse;
}

export class UnimplementedCaptureService implements CaptureService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  open(req: strims_video_v1_CaptureOpenRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureOpenResponse> | strims_video_v1_CaptureOpenResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  update(req: strims_video_v1_CaptureUpdateRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureUpdateResponse> | strims_video_v1_CaptureUpdateResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  append(req: strims_video_v1_CaptureAppendRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureAppendResponse> | strims_video_v1_CaptureAppendResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  close(req: strims_video_v1_CaptureCloseRequest, call: strims_rpc_Call): Promise<strims_video_v1_CaptureCloseResponse> | strims_video_v1_CaptureCloseResponse { throw new Error("not implemented"); }
}

export const registerCaptureService = (host: strims_rpc_Service, service: CaptureService): void => {
  host.registerMethod<strims_video_v1_CaptureOpenRequest, strims_video_v1_CaptureOpenResponse>("strims.video.v1.Capture.Open", (req, call) => service.open(req, call), strims_video_v1_CaptureOpenRequest);
  host.registerMethod<strims_video_v1_CaptureUpdateRequest, strims_video_v1_CaptureUpdateResponse>("strims.video.v1.Capture.Update", (req, call) => service.update(req, call), strims_video_v1_CaptureUpdateRequest);
  host.registerMethod<strims_video_v1_CaptureAppendRequest, strims_video_v1_CaptureAppendResponse>("strims.video.v1.Capture.Append", (req, call) => service.append(req, call), strims_video_v1_CaptureAppendRequest);
  host.registerMethod<strims_video_v1_CaptureCloseRequest, strims_video_v1_CaptureCloseResponse>("strims.video.v1.Capture.Close", (req, call) => service.close(req, call), strims_video_v1_CaptureCloseRequest);
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

