import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";
import { Base64 } from "js-base64";

import * as directoryv1 from "../../../apis/strims/network/v1/directory/directory";
import { DirectoryFrontendService } from "../../../apis/strims/network/v1/directory/directory_rpc";
import events from "./events";

export default class DirectoryService implements DirectoryFrontendService {
  responses: Readable<directoryv1.FrontendOpenResponse>;

  constructor(
    responses: Readable<directoryv1.FrontendOpenResponse> = new PassThrough({ objectMode: true })
  ) {
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

  test(req: directoryv1.FrontendTestRequest): Promise<directoryv1.FrontendTestResponse> {
    console.log("test", req);
    return Promise.resolve(new directoryv1.FrontendTestResponse());
  }

  publish(req: directoryv1.FrontendPublishRequest): Promise<directoryv1.FrontendPublishResponse> {
    console.log("publish", req);
    return Promise.resolve(new directoryv1.FrontendPublishResponse());
  }

  unpublish(
    req: directoryv1.FrontendUnpublishRequest
  ): Promise<directoryv1.FrontendUnpublishResponse> {
    console.log("unpublish", req);
    return Promise.resolve(new directoryv1.FrontendUnpublishResponse());
  }

  join(req: directoryv1.FrontendJoinRequest): Promise<directoryv1.FrontendJoinResponse> {
    console.log("join", req);
    return Promise.resolve(new directoryv1.FrontendJoinResponse());
  }

  part(req: directoryv1.FrontendPartRequest): Promise<directoryv1.FrontendPartResponse> {
    console.log("part", req);
    return Promise.resolve(new directoryv1.FrontendPartResponse());
  }

  getListingRecord(
    req: directoryv1.FrontendGetListingRecordRequest
  ): Promise<directoryv1.FrontendGetListingRecordResponse> {
    return Promise.reject("not implemented");
  }

  listListingRecords(
    req: directoryv1.FrontendListListingRecordsRequest
  ): Promise<directoryv1.FrontendListListingRecordsResponse> {
    return Promise.reject("not implemented");
  }

  updateListingRecord(
    req: directoryv1.FrontendUpdateListingRecordRequest
  ): Promise<directoryv1.FrontendUpdateListingRecordResponse> {
    return Promise.reject("not implemented");
  }
}
