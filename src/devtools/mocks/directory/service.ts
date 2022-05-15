// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { Base64 } from "js-base64";

import * as directoryv1 from "../../../apis/strims/network/v1/directory/directory";
import {
  DirectoryFrontendService,
  UnimplementedDirectoryFrontendService,
} from "../../../apis/strims/network/v1/directory/directory_rpc";
import events from "./events";

export default class DirectoryService
  extends UnimplementedDirectoryFrontendService
  implements DirectoryFrontendService
{
  responses: Readable<directoryv1.FrontendOpenResponse>;

  constructor(
    responses: Readable<directoryv1.FrontendOpenResponse> = new PassThrough({ objectMode: true })
  ) {
    super();
    this.responses = responses;
  }

  destroy(): void {
    this.responses.destroy();
  }

  emitOpenResponse(broadcast: directoryv1.FrontendOpenResponse): void {
    this.responses.push(broadcast);
  }

  open(): Readable<directoryv1.FrontendOpenResponse> {
    const ch = new PassThrough({ objectMode: true });

    window.setTimeout(() => {
      for (let i = 0; i < events.length; i++) {
        ch.push(
          new directoryv1.FrontendOpenResponse({
            networkKey: Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE="),
            body: {
              broadcast: {
                events: [
                  {
                    body: {
                      listingChange: events[i],
                    },
                  },
                ],
              },
            },
          })
        );
      }
    });

    this.responses.on("data", (response) => ch.push(response));

    return ch;
  }

  test(req: directoryv1.FrontendTestRequest): directoryv1.FrontendTestResponse {
    console.log("test", req);
    return new directoryv1.FrontendTestResponse();
  }

  publish(req: directoryv1.FrontendPublishRequest): directoryv1.FrontendPublishResponse {
    console.log("publish", req);
    return new directoryv1.FrontendPublishResponse();
  }

  unpublish(req: directoryv1.FrontendUnpublishRequest): directoryv1.FrontendUnpublishResponse {
    console.log("unpublish", req);
    return new directoryv1.FrontendUnpublishResponse();
  }

  join(req: directoryv1.FrontendJoinRequest): directoryv1.FrontendJoinResponse {
    console.log("join", req);
    return new directoryv1.FrontendJoinResponse();
  }

  part(req: directoryv1.FrontendPartRequest): directoryv1.FrontendPartResponse {
    console.log("part", req);
    return new directoryv1.FrontendPartResponse();
  }
}
