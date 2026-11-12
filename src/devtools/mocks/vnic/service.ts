// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import * as vnicv1 from "../../../apis/strims/vnic/v1/vnic";
import { Config } from "../../../apis/strims/vnic/v1/vnic";
import {
  UnimplementedVNICFrontendService,
  VNICFrontendService,
} from "../../../apis/strims/vnic/v1/vnic_rpc";

// 50 Mbit/s and 25 peers.
export const defaultConfig = (): Config =>
  new Config({ maxUploadBytesPerSecond: BigInt((50 * Math.pow(2, 20)) / 8), maxPeers: 25 });

// The node's VNIC config in memory.
export default class VNICService
  extends UnimplementedVNICFrontendService
  implements VNICFrontendService
{
  config: Config;

  constructor(config: Config = defaultConfig()) {
    super();
    this.config = config;
  }

  getConfig(): vnicv1.GetConfigResponse {
    return new vnicv1.GetConfigResponse({ config: this.config });
  }

  setConfig(req: vnicv1.SetConfigRequest): vnicv1.SetConfigResponse {
    this.config = new Config(req.config);
    return new vnicv1.SetConfigResponse({ config: this.config });
  }
}
