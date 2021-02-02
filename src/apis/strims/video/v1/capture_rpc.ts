import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

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

registerType("strims.video.v1.CaptureOpenRequest", CaptureOpenRequest);
registerType("strims.video.v1.CaptureOpenResponse", CaptureOpenResponse);
registerType("strims.video.v1.CaptureUpdateRequest", CaptureUpdateRequest);
registerType("strims.video.v1.CaptureUpdateResponse", CaptureUpdateResponse);
registerType("strims.video.v1.CaptureAppendRequest", CaptureAppendRequest);
registerType("strims.video.v1.CaptureAppendResponse", CaptureAppendResponse);
registerType("strims.video.v1.CaptureCloseRequest", CaptureCloseRequest);
registerType("strims.video.v1.CaptureCloseResponse", CaptureCloseResponse);

export class CaptureClient {
  constructor(private readonly host: RPCHost) {}

  public open(arg: ICaptureOpenRequest = new CaptureOpenRequest()): Promise<CaptureOpenResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Open", new CaptureOpenRequest(arg)));
  }

  public update(arg: ICaptureUpdateRequest = new CaptureUpdateRequest()): Promise<CaptureUpdateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Update", new CaptureUpdateRequest(arg)));
  }

  public append(arg: ICaptureAppendRequest = new CaptureAppendRequest()): Promise<CaptureAppendResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Append", new CaptureAppendRequest(arg)));
  }

  public close(arg: ICaptureCloseRequest = new CaptureCloseRequest()): Promise<CaptureCloseResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.Capture.Close", new CaptureCloseRequest(arg)));
  }
}

