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

  test(req: directoryv1.FrontendTestRequest) {
    console.log("test", req);
    return new directoryv1.FrontendTestResponse();
  }

  publish(req: directoryv1.FrontendPublishRequest) {
    console.log("publish", req);
    return new directoryv1.FrontendPublishResponse();
  }

  unpublish(req: directoryv1.FrontendUnpublishRequest) {
    console.log("unpublish", req);
    return new directoryv1.FrontendUnpublishResponse();
  }

  join(req: directoryv1.FrontendJoinRequest) {
    console.log("join", req);
    return new directoryv1.FrontendJoinResponse();
  }

  part(req: directoryv1.FrontendPartRequest) {
    console.log("part", req);
    return new directoryv1.FrontendPartResponse();
  }

  moderateListing(req: directoryv1.FrontendModerateListingRequest) {
    console.log("moderateListing", req);
    return new directoryv1.FrontendModerateListingResponse();
  }

  moderateUser(req: directoryv1.FrontendModerateUserRequest) {
    console.log("moderateUser", req);
    return new directoryv1.FrontendModerateUserResponse();
  }

  getUsers(req: directoryv1.FrontendGetUsersRequest) {
    console.log("getUsers", req);
    return new directoryv1.FrontendGetUsersResponse();
  }

  getListings(req: directoryv1.FrontendGetListingsRequest) {
    console.log("getListings", req);
    return new directoryv1.FrontendGetListingsResponse();
  }

  watchListingUsers(req: directoryv1.FrontendWatchListingUsersRequest) {
    console.log("watchListingUsers", req);
    const ch = new PassThrough({ objectMode: true });
    return ch;
  }
}
