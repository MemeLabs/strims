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
import imgAOCFace from "../chat/emotes/static/AOCFace.png";
import imgBasedGod from "../chat/emotes/static/BasedGod.png";
import imgBOOMER from "../chat/emotes/static/BOOMER.png";
import imgComfyApe from "../chat/emotes/static/ComfyApe.png";
import imgComfyAYA from "../chat/emotes/static/ComfyAYA.png";
import imgComfyCat from "../chat/emotes/static/ComfyCat.png";
import imgComfyDog from "../chat/emotes/static/ComfyDog.png";
import imgComfyFeels from "../chat/emotes/static/ComfyFeels.png";
import imgComfyWeird from "../chat/emotes/static/ComfyWeird.png";
import imgDOUBT from "../chat/emotes/static/DOUBT.png";
import imgFeelsStrongMan from "../chat/emotes/static/FeelsStrongMan.png";
import imghaHAA from "../chat/emotes/static/haHAA.png";
import imgHmmm from "../chat/emotes/static/Hmmm.png";
import imgINFESTOR from "../chat/emotes/static/INFESTOR.png";
import imgMiyanoComfy from "../chat/emotes/static/MiyanoComfy.png";
import imgmonkaVirus from "../chat/emotes/static/monkaVirus.png";
import imgMOOBERS from "../chat/emotes/static/MOOBERS.png";
import imgNeneLaugh from "../chat/emotes/static/NeneLaugh.png";
import imgNoTears from "../chat/emotes/static/NoTears.png";
import imgNotLikeThis from "../chat/emotes/static/NotLikeThis.png";
import imgOnlyPretending from "../chat/emotes/static/OnlyPretending.png";
import imgORDAH from "../chat/emotes/static/ORDAH.png";
import imgOverRustle from "../chat/emotes/static/OverRustle.png";
import imgPAIN from "../chat/emotes/static/PAIN.png";
import imgPauseChamp from "../chat/emotes/static/PauseChamp.png";
import imgPepeHands from "../chat/emotes/static/PepeHands.png";
import imgPepeLaugh from "../chat/emotes/static/PepeLaugh.png";
import imgPepeMods from "../chat/emotes/static/PepeMods.png";
import imgPepoComfy from "../chat/emotes/static/PepoComfy.png";
import imgQUEEN from "../chat/emotes/static/QUEEN.png";
import imgZOOMER from "../chat/emotes/static/ZOOMER.png";

const images = [
  imgAOCFace,
  imgBasedGod,
  imgBOOMER,
  imgComfyApe,
  imgComfyAYA,
  imgComfyCat,
  imgComfyDog,
  imgComfyFeels,
  imgComfyWeird,
  imgDOUBT,
  imgFeelsStrongMan,
  imghaHAA,
  imgHmmm,
  imgINFESTOR,
  imgMiyanoComfy,
  imgmonkaVirus,
  imgMOOBERS,
  imgNeneLaugh,
  imgNoTears,
  imgNotLikeThis,
  imgOnlyPretending,
  imgORDAH,
  imgOverRustle,
  imgPAIN,
  imgPauseChamp,
  imgPepeHands,
  imgPepeLaugh,
  imgPepeMods,
  imgPepoComfy,
  imgQUEEN,
  imgZOOMER,
];

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
  "icon": {
    "type": "image/png",
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

    let i = 0;
    for (const url of images.slice(0, this.limit)) {
      void (async () => {
        const res = await fetch(url);
        const buf = await res.arrayBuffer();

        i++;
        network.id = BigInt(i);
        network.icon.data = new Uint8Array(buf);

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
      })();
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
