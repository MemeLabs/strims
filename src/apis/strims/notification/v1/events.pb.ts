// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  Notification as strims_notification_v1_Notification,
  type INotification as strims_notification_v1_INotification,
} from "./notification.pb";

export type INotificationChangeEvent = {
  notification?: strims_notification_v1_INotification;
}

export class NotificationChangeEvent {
  notification: strims_notification_v1_Notification | undefined;

  constructor(v?: INotificationChangeEvent) {
    this.notification = v?.notification && new strims_notification_v1_Notification(v.notification);
  }

  static encode(m: NotificationChangeEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.notification) strims_notification_v1_Notification.encode(m.notification, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NotificationChangeEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NotificationChangeEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.notification = strims_notification_v1_Notification.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

export type INotificationDeleteEvent = {
  notification?: strims_notification_v1_INotification;
}

export class NotificationDeleteEvent {
  notification: strims_notification_v1_Notification | undefined;

  constructor(v?: INotificationDeleteEvent) {
    this.notification = v?.notification && new strims_notification_v1_Notification(v.notification);
  }

  static encode(m: NotificationDeleteEvent, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.notification) strims_notification_v1_Notification.encode(m.notification, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): NotificationDeleteEvent {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new NotificationDeleteEvent();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.notification = strims_notification_v1_Notification.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7, tag >>> 3);
        break;
      }
    }
    r.leave(parentEnd);
    return m;
  }
}

