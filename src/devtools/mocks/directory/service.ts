// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { Base64 } from "js-base64";

import * as directoryv1 from "../../../apis/strims/network/v1/directory/directory";
import {
  DirectoryFrontendService,
  UnimplementedDirectoryFrontendService,
} from "../../../apis/strims/network/v1/directory/directory_rpc";
import { ImageType } from "../../../apis/strims/type/image";
import images from "../images";
import events from "./events";

export default class DirectoryService
  extends UnimplementedDirectoryFrontendService
  implements DirectoryFrontendService
{
  constructor() {
    super();
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
    return new directoryv1.FrontendGetListingsResponse({
      listings: [
        {
          network: {
            id: BigInt(1),
            key: Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE="),
            name: "test",
          },
          listings: events.map((e) => ({
            ...e,
            userCount: 0,
          })),
        },
      ],
    });
  }

  watchListings(req: directoryv1.FrontendWatchListingsRequest) {
    console.log("watchListings", req);
    const ch = new PassThrough({ objectMode: true });
    ch.push(
      new directoryv1.FrontendWatchListingsResponse({
        events: [
          {
            event: {
              change: {
                listings: {
                  network: {
                    id: BigInt(1),
                    key: Base64.toUint8Array("cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE="),
                    name: "test",
                  },
                  listings: events.map((e) => ({
                    ...e,
                    moderation: {},
                    userCount: 0,
                    recentUserCount: 0,
                  })),
                },
              },
            },
          },
        ],
      })
    );
    return ch;
  }

  watchListingUsers(req: directoryv1.FrontendWatchListingUsersRequest) {
    console.log("watchListingUsers", req);
    const ch = new PassThrough({ objectMode: true });
    ch.push(new directoryv1.FrontendWatchListingUsersResponse({}));
    return ch;
  }

  watchAssetBundles() {
    const ch = new PassThrough({ objectMode: true });

    let i = 0;
    for (const url of images) {
      void (async () => {
        const res = await fetch(url);
        const buf = await res.arrayBuffer();

        ch.push(
          new directoryv1.FrontendWatchAssetBundlesResponse({
            networkId: BigInt(i++),
            assetBundle: {
              icon: {
                type: ImageType.IMAGE_TYPE_PNG,
                data: new Uint8Array(buf),
              },
            },
          })
        );
      })();
    }

    return ch;
  }
}
