// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type INotification = {
  id?: bigint;
  createdAt?: bigint;
  status?: Notification_Status;
  title?: string;
  message?: string;
  subject?: Notification_ISubject;
  errorCode?: number;
}

export class Notification {
  id: bigint;
  createdAt: bigint;
  status: Notification_Status;
  title: string;
  message: string;
  subject: Notification_Subject | undefined;
  errorCode: number;

  constructor(v?: INotification) {
    this.id = v?.id || BigInt(0);
    this.createdAt = v?.createdAt || BigInt(0);
    this.status = v?.status || 0;
    this.title = v?.title || "";
    this.message = v?.message || "";
    this.subject = v?.subject && new Notification_Subject(v.subject);
    this.errorCode = v?.errorCode || 0;
  }

  static encode(m: Notification, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.createdAt) w.uint32(16).int64(m.createdAt);
    if (m.status) w.uint32(24).uint32(m.status);
    if (m.title.length) w.uint32(34).string(m.title);
    if (m.message.length) w.uint32(42).string(m.message);
    if (m.subject) Notification_Subject.encode(m.subject, w.uint32(50).fork()).ldelim();
    if (m.errorCode) w.uint32(56).int32(m.errorCode);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Notification {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Notification();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.id = r.uint64();
        break;
        case 2:
        m.createdAt = r.int64();
        break;
        case 3:
        m.status = r.uint32();
        break;
        case 4:
        m.title = r.string();
        break;
        case 5:
        m.message = r.string();
        break;
        case 6:
        m.subject = Notification_Subject.decode(r, r.uint32());
        break;
        case 7:
        m.errorCode = r.int32();
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

export type Notification_ISubject = {
  model?: Notification_Subject_Model;
  id?: bigint;
}

export class Notification_Subject {
  model: Notification_Subject_Model;
  id: bigint;

  constructor(v?: Notification_ISubject) {
    this.model = v?.model || 0;
    this.id = v?.id || BigInt(0);
  }

  static encode(m: Notification_Subject, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.model) w.uint32(8).uint32(m.model);
    if (m.id) w.uint32(16).uint64(m.id);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Notification_Subject {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Notification_Subject();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.model = r.uint32();
        break;
        case 2:
        m.id = r.uint64();
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

export enum Notification_Subject_Model {
  NOTIFICATION_SUBJECT_MODEL_NETWORK = 0,
}

export enum Notification_Status {
  STATUS_INFO = 0,
  STATUS_SUCCESS = 1,
  STATUS_WARNING = 2,
  STATUS_ERROR = 3,
}

export type IEvent = {
  body?: Event_IBody
}

export class Event {
  body: Event_TBody;

  constructor(v?: IEvent) {
    this.body = v?.body ? new Event_Body(v.body) : new Event_Body();
  }

  static encode(m: Event, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case Event_BodyCase.NOTIFICATION:
      Notification.encode(m.body.notification, w.uint32(8010).fork()).ldelim();
      break;
      case Event_BodyCase.DISMISS:
      w.uint32(8016).uint64(m.body.dismiss);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new Event();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new Event_Body({ notification: Notification.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new Event_Body({ dismiss: r.uint64() });
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

export enum Event_BodyCase {
  NOT_SET = 0,
  NOTIFICATION = 1001,
  DISMISS = 1002,
}

export type Event_IBody =
{ case?: Event_BodyCase.NOT_SET }
|{ case?: Event_BodyCase.NOTIFICATION, notification: INotification }
|{ case?: Event_BodyCase.DISMISS, dismiss: bigint }
;

export type Event_TBody = Readonly<
{ case: Event_BodyCase.NOT_SET }
|{ case: Event_BodyCase.NOTIFICATION, notification: Notification }
|{ case: Event_BodyCase.DISMISS, dismiss: bigint }
>;

class Event_BodyImpl {
  declare notification?: Notification;
  declare dismiss?: bigint;
  case: Event_BodyCase = Event_BodyCase.NOT_SET;

  constructor(v?: Event_IBody) {
    if (v && "notification" in v) {
      this.case = Event_BodyCase.NOTIFICATION;
      this.notification = new Notification(v.notification);
    } else
    if (v && "dismiss" in v) {
      this.case = Event_BodyCase.DISMISS;
      this.dismiss = v.dismiss;
    }
  }
}

export const Event_Body = Event_BodyImpl as {
  new (): Readonly<{ case: Event_BodyCase.NOT_SET }>;
  new <T extends Event_IBody>(v: T): Readonly<
  T extends { notification: INotification } ? { case: Event_BodyCase.NOTIFICATION, notification: Notification } :
  T extends { dismiss: bigint } ? { case: Event_BodyCase.DISMISS, dismiss: bigint } :
  never
  >;
};

export type IWatchRequest = object;

export class WatchRequest {

  constructor(v?: IWatchRequest) {
    void v; // an empty message has no fields to set
  }

  static encode(m: WatchRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchRequest {
    if (r instanceof Reader && length) r.skip(length);
    return new WatchRequest();
  }
}

export type IWatchResponse = {
  event?: IEvent;
}

export class WatchResponse {
  event: Event | undefined;

  constructor(v?: IWatchResponse) {
    this.event = v?.event && new Event(v.event);
  }

  static encode(m: WatchResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.event) Event.encode(m.event, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new WatchResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.event = Event.decode(r, r.uint32());
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

export type IDismissRequest = {
  ids?: bigint[];
}

export class DismissRequest {
  ids: bigint[];

  constructor(v?: IDismissRequest) {
    this.ids = v?.ids ? v.ids : [];
  }

  static encode(m: DismissRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    m.ids.reduce((w, v) => w.uint64(v), w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DismissRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new DismissRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.ids.push(r.uint64());
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

export type IDismissResponse = object;

export class DismissResponse {

  constructor(v?: IDismissResponse) {
    void v; // an empty message has no fields to set
  }

  static encode(m: DismissResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): DismissResponse {
    if (r instanceof Reader && length) r.skip(length);
    return new DismissResponse();
  }
}

