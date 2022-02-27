import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";

import * as videov1 from "../../../apis/strims/video/v1/egress";
import { EgressService } from "../../../apis/strims/video/v1/egress_rpc";
import vid0 from "./sample/0.m4s";
import vid1 from "./sample/1.m4s";
import vid2 from "./sample/2.m4s";
import vid3 from "./sample/3.m4s";
import vid4 from "./sample/4.m4s";
import vid5 from "./sample/5.m4s";
import vid6 from "./sample/6.m4s";
import vid7 from "./sample/7.m4s";
import vid8 from "./sample/8.m4s";
import vid9 from "./sample/9.m4s";
import vidInit from "./sample/init.mp4";

export default class VideoService implements EgressService {
  destroy(): void {
    return null;
  }

  openStream(req): Readable<videov1.EgressOpenStreamResponse> {
    const ch = new PassThrough({ objectMode: true });

    ch.push(
      new videov1.EgressOpenStreamResponse({
        body: new videov1.EgressOpenStreamResponse.Body({
          open: new videov1.EgressOpenStreamResponse.Open({
            transferId: new Uint8Array(),
          }),
        }),
      })
    );

    void (async () => {
      const init = await fetch(vidInit)
        .then((res) => res.arrayBuffer())
        .then((b) => new Uint8Array(b));
      const segments = await Promise.all(
        [vid0, vid1, vid2, vid3, vid4, vid5, vid6, vid7, vid8, vid9].map(async (u) => {
          const b = await fetch(u)
            .then((res) => res.arrayBuffer())
            .then((b) => new Uint8Array(b));
          const seg = new Uint8Array(2 + init.length + b.length);
          seg[0] = (init.length >> 8) & 0xff;
          seg[1] = init.length & 0xff;
          seg.set(init, 2);
          seg.set(b, 2 + init.length);
          return seg;
        })
      );

      let i = 0;
      const skip = [false, false, false, true, false, false, true, false, false, false];
      const ivl = setInterval(() => {
        ch.push(
          new videov1.EgressOpenStreamResponse({
            body: new videov1.EgressOpenStreamResponse.Body({
              data: new videov1.EgressOpenStreamResponse.Data({
                data: segments[i],
                segmentEnd: true,
                // bufferUnderrun: skip[i],
              }),
            }),
          })
        );
        if (i++ >= segments.length) {
          clearInterval(ivl);
        }
      }, 3000);
    })();

    return ch;
  }
}
