import { RPCHost } from "@memelabs/protobuf/lib/rpc/host";
import { registerType } from "@memelabs/protobuf/lib/rpc/registry";

import {
  IVideoChannelListRequest,
  VideoChannelListRequest,
  VideoChannelListResponse,
  IVideoChannelCreateRequest,
  VideoChannelCreateRequest,
  VideoChannelCreateResponse,
  IVideoChannelUpdateRequest,
  VideoChannelUpdateRequest,
  VideoChannelUpdateResponse,
  IVideoChannelDeleteRequest,
  VideoChannelDeleteRequest,
  VideoChannelDeleteResponse,
} from "./channel";

registerType("strims.video.v1.VideoChannelListRequest", VideoChannelListRequest);
registerType("strims.video.v1.VideoChannelListResponse", VideoChannelListResponse);
registerType("strims.video.v1.VideoChannelCreateRequest", VideoChannelCreateRequest);
registerType("strims.video.v1.VideoChannelCreateResponse", VideoChannelCreateResponse);
registerType("strims.video.v1.VideoChannelUpdateRequest", VideoChannelUpdateRequest);
registerType("strims.video.v1.VideoChannelUpdateResponse", VideoChannelUpdateResponse);
registerType("strims.video.v1.VideoChannelDeleteRequest", VideoChannelDeleteRequest);
registerType("strims.video.v1.VideoChannelDeleteResponse", VideoChannelDeleteResponse);

export class VideoChannelFrontendClient {
  constructor(private readonly host: RPCHost) {}

  public list(arg?: IVideoChannelListRequest): Promise<VideoChannelListResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.List", new VideoChannelListRequest(arg)));
  }

  public create(arg?: IVideoChannelCreateRequest): Promise<VideoChannelCreateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Create", new VideoChannelCreateRequest(arg)));
  }

  public update(arg?: IVideoChannelUpdateRequest): Promise<VideoChannelUpdateResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Update", new VideoChannelUpdateRequest(arg)));
  }

  public delete(arg?: IVideoChannelDeleteRequest): Promise<VideoChannelDeleteResponse> {
    return this.host.expectOne(this.host.call("strims.video.v1.VideoChannelFrontend.Delete", new VideoChannelDeleteRequest(arg)));
  }
}

