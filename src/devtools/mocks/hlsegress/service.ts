// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as hlsv1 from "../../../apis/strims/video/v1/hls_egress";
import {
  HLSEgressService as HLSEgressServiceInterface,
  UnimplementedHLSEgressService,
} from "../../../apis/strims/video/v1/hls_egress_rpc";

export interface HLSEgressServiceOptions {
  /** What isSupported answers; default true. */
  supported?: boolean;
  /** The stored config; default enabled at https://cdn.strims.gg. */
  config?: hlsv1.IHLSEgressConfig;
}

// One node's HLS egress settings.
export default class HLSEgressService
  extends UnimplementedHLSEgressService
  implements HLSEgressServiceInterface
{
  supported: boolean;
  config: hlsv1.HLSEgressConfig;

  constructor({ supported = true, config }: HLSEgressServiceOptions = {}) {
    super();
    this.supported = supported;
    this.config = new hlsv1.HLSEgressConfig(
      config ?? { enabled: true, publicServerAddr: "https://cdn.strims.gg" }
    );
  }

  // A union, so a story's subclass can answer late.
  isSupported(): Promise<hlsv1.HLSEgressIsSupportedResponse> | hlsv1.HLSEgressIsSupportedResponse {
    return new hlsv1.HLSEgressIsSupportedResponse({ supported: this.supported });
  }

  getConfig(): hlsv1.HLSEgressGetConfigResponse {
    return new hlsv1.HLSEgressGetConfigResponse({ config: this.config });
  }

  setConfig(req: hlsv1.HLSEgressSetConfigRequest): hlsv1.HLSEgressSetConfigResponse {
    this.config = new hlsv1.HLSEgressConfig(req.config);
    return new hlsv1.HLSEgressSetConfigResponse({ config: this.config });
  }
}
