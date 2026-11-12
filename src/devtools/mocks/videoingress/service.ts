// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";

import * as ingressv1 from "../../../apis/strims/video/v1/ingress";
import {
  UnimplementedVideoIngressService,
  VideoIngressService as VideoIngressServiceInterface,
} from "../../../apis/strims/video/v1/ingress_rpc";
import { bytes } from "../../storybook/fixtures/ids";

export interface VideoIngressServiceOptions {
  /** What isSupported answers; default true. */
  supported?: boolean;
  /** The stored config; default enabled, serving on 0.0.0.0:1935 as ingress.strims.gg. */
  config?: ingressv1.IVideoIngressConfig;
}

// A channel's stream key as the node formats it: the id and a signature,
// unpadded base64url.
export const streamKey = (channelId: bigint): string =>
  Base64.fromUint8Array(bytes(`stream-key:${channelId}`, 65), true);

// One node's RTMP ingress: its config, and channel URLs on its address.
export default class VideoIngressService
  extends UnimplementedVideoIngressService
  implements VideoIngressServiceInterface
{
  supported: boolean;
  config: ingressv1.VideoIngressConfig;

  constructor({ supported = true, config }: VideoIngressServiceOptions = {}) {
    super();
    this.supported = supported;
    this.config = new ingressv1.VideoIngressConfig(
      config ?? {
        enabled: true,
        serverAddr: "0.0.0.0:1935",
        publicServerAddr: "ingress.strims.gg",
      }
    );
  }

  // A union, so a story's subclass can answer late.
  isSupported():
    | Promise<ingressv1.VideoIngressIsSupportedResponse>
    | ingressv1.VideoIngressIsSupportedResponse {
    return new ingressv1.VideoIngressIsSupportedResponse({ supported: this.supported });
  }

  getConfig(): ingressv1.VideoIngressGetConfigResponse {
    return new ingressv1.VideoIngressGetConfigResponse({ config: this.config });
  }

  setConfig(req: ingressv1.VideoIngressSetConfigRequest): ingressv1.VideoIngressSetConfigResponse {
    this.config = new ingressv1.VideoIngressConfig(req.config);
    return new ingressv1.VideoIngressSetConfigResponse({ config: this.config });
  }

  listStreams(): ingressv1.VideoIngressListStreamsResponse {
    return new ingressv1.VideoIngressListStreamsResponse();
  }

  getChannelURL(
    req: ingressv1.VideoIngressGetChannelURLRequest
  ): ingressv1.VideoIngressGetChannelURLResponse {
    const serverAddr = this.config.publicServerAddr || this.config.serverAddr;
    const key = streamKey(req.id);
    return new ingressv1.VideoIngressGetChannelURLResponse({
      url: `rtmp://${serverAddr}/live/${key}`,
      serverAddr,
      streamKey: key,
    });
  }
}
