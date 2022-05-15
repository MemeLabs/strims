// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import { Readable } from "@memelabs/protobuf/lib/rpc/stream";

import * as notificationv1 from "../../../apis/strims/notification/v1/notification";
import { NotificationFrontendService } from "../../../apis/strims/notification/v1/notification_rpc";

export default class NotificationService implements NotificationFrontendService {
  events: Readable<notificationv1.Event>;

  constructor() {
    this.events = new PassThrough({ objectMode: true });
  }

  destroy(): void {
    this.events.destroy();
  }

  emitEvent(event: notificationv1.Event): void {
    this.events.push(event);
  }

  watch(): Readable<notificationv1.WatchResponse> {
    const ch = new PassThrough({ objectMode: true });

    this.events.on("data", (event) => ch.push(new notificationv1.WatchResponse({ event })));

    return ch;
  }

  dismiss(req: notificationv1.DismissRequest): Promise<notificationv1.DismissResponse> {
    for (const id of req.ids) {
      this.events.push(
        new notificationv1.Event({
          body: new notificationv1.Event.Body({ dismiss: id }),
        })
      );
    }

    return Promise.resolve(new notificationv1.DismissResponse());
  }
}
