// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { Base64 } from "js-base64";

import * as networkv1 from "../../../apis/strims/network/v1/network";
import { Network } from "../../../apis/strims/network/v1/network";
import {
  NetworkFrontendService,
  UnimplementedNetworkFrontendService,
} from "../../../apis/strims/network/v1/network_rpc";
import images from "../images";

const network = new Network({
  "id": BigInt("3501"),
  "certificate": {
    "key": Base64.toUint8Array("1cFuxhf5HI5d6XQgypXXfqI8LducWmeJtHpUIADR1aU="),
    "keyType": 1,
    "keyUsage": 5,
    "subject": "alias",
    "notBefore": BigInt("1633572461"),
    "notAfter": BigInt("1696648061"),
    "serialNumber": Base64.toUint8Array("qBdmT/tj/b5x5m9ENRPLUQ=="),
    "signature": Base64.toUint8Array(
      "ND2uEYXMTVy6820/ql4bJ2qdexlRVK+qKB+NKyAO5k3ngyr8k6rVl+ez7OxUiupcm+z42N2ln43rhPpnECMWDw=="
    ),
    "parentOneof": {
      "case": 9,
      "parent": {
        "key": Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE="),
        "keyType": 1,
        "keyUsage": 4,
        "subject": "Test Network",
        "notBefore": BigInt("1633572461"),
        "notAfter": BigInt("1696648061"),
        "serialNumber": Base64.toUint8Array("L1mJl1pJAhGhbeWNZuTKxw=="),
        "signature": Base64.toUint8Array(
          "tjOLhRDUr+JOgi1nz4jmwjghS8ccAL63SItgm0hvS64ptKZmx0FD5e4UXaOILJfs1yx496eewoyZBNAIqDiPAA=="
        ),
      },
    },
  },
});

export default class NetworkService
  extends UnimplementedNetworkFrontendService
  implements NetworkFrontendService
{
  constructor(private limit: number = Infinity) {
    super();
  }

  public watch(): Readable<networkv1.WatchNetworksResponse> {
    const ch = new PassThrough({ objectMode: true });

    for (let i = 0; i < 30; i++) {
      network.id = BigInt(i);

      ch.push(
        new networkv1.WatchNetworksResponse({
          event: new networkv1.NetworkEvent({
            body: new networkv1.NetworkEvent.Body({
              networkStart: new networkv1.NetworkEvent.NetworkStart({
                network,
                peerCount: Math.pow(2, i),
              }),
            }),
          }),
        })
      );
    }

    return ch;
  }

  getUIConfig(): networkv1.GetUIConfigResponse {
    const networkDisplayOrder: bigint[] = [];
    for (let i = 0; i < Math.min(images.length, this.limit); i++) {
      networkDisplayOrder.push(BigInt(i));
    }

    return new networkv1.GetUIConfigResponse({
      config: {
        networkDisplayOrder,
      },
    });
  }
}
