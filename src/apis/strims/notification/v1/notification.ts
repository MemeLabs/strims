import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type INotification = {
  id?: bigint;
  createdAt?: bigint;
  status?: strims_notification_v1_Notification_Status;
  title?: string;
  message?: string;
  subject?: strims_notification_v1_Notification_ISubject;
  errorCode?: number;
}

export class Notification {
  id: bigint;
  createdAt: bigint;
  status: strims_notification_v1_Notification_Status;
  title: string;
  message: string;
  subject: strims_notification_v1_Notification_Subject | undefined;
  errorCode: number;

  constructor(v?: INotification) {
    this.id = v?.id || BigInt(0);
    this.createdAt = v?.createdAt || BigInt(0);
    this.status = v?.status || 0;
    this.title = v?.title || "";
    this.message = v?.message || "";
    this.subject = v?.subject && new strims_notification_v1_Notification_Subject(v.subject);
    this.errorCode = v?.errorCode || 0;
  }

  static encode(m: Notification, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.id) w.uint32(8).uint64(m.id);
    if (m.createdAt) w.uint32(16).int64(m.createdAt);
    if (m.status) w.uint32(24).uint32(m.status);
    if (m.title.length) w.uint32(34).string(m.title);
    if (m.message.length) w.uint32(42).string(m.message);
    if (m.subject) strims_notification_v1_Notification_Subject.encode(m.subject, w.uint32(50).fork()).ldelim();
    if (m.errorCode) w.uint32(56).int32(m.errorCode);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Notification {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
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
        m.subject = strims_notification_v1_Notification_Subject.decode(r, r.uint32());
        break;
        case 7:
        m.errorCode = r.int32();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Notification {
  export type ISubject = {
    model?: strims_notification_v1_Notification_Subject_Model;
    id?: bigint;
  }

  export class Subject {
    model: strims_notification_v1_Notification_Subject_Model;
    id: bigint;

    constructor(v?: ISubject) {
      this.model = v?.model || 0;
      this.id = v?.id || BigInt(0);
    }

    static encode(m: Subject, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.model) w.uint32(8).uint32(m.model);
      if (m.id) w.uint32(16).uint64(m.id);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Subject {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Subject();
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
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export namespace Subject {
    export enum Model {
      NOTIFICATION_SUBJECT_MODEL_NETWORK = 0,
    }
  }

  export enum Status {
    STATUS_INFO = 0,
    STATUS_SUCCESS = 1,
    STATUS_WARNING = 2,
    STATUS_ERROR = 3,
  }
}

export type IEvent = {
  body?: Event.IBody
}

export class Event {
  body: Event.TBody;

  constructor(v?: IEvent) {
    this.body = new Event.Body(v?.body);
  }

  static encode(m: Event, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case Event.BodyCase.NOTIFICATION:
      strims_notification_v1_Notification.encode(m.body.notification, w.uint32(8010).fork()).ldelim();
      break;
      case Event.BodyCase.DISMISS:
      w.uint32(8016).uint64(m.body.dismiss);
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): Event {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new Event();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new Event.Body({ notification: strims_notification_v1_Notification.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new Event.Body({ dismiss: r.uint64() });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace Event {
  export enum BodyCase {
    NOT_SET = 0,
    NOTIFICATION = 1001,
    DISMISS = 1002,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.NOTIFICATION, notification: strims_notification_v1_INotification }
  |{ case?: BodyCase.DISMISS, dismiss: bigint }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.NOTIFICATION, notification: strims_notification_v1_Notification }
  |{ case: BodyCase.DISMISS, dismiss: bigint }
  >;

  class BodyImpl {
    notification: strims_notification_v1_Notification;
    dismiss: bigint;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "notification" in v) {
        this.case = BodyCase.NOTIFICATION;
        this.notification = new strims_notification_v1_Notification(v.notification);
      } else
      if (v && "dismiss" in v) {
        this.case = BodyCase.DISMISS;
        this.dismiss = v.dismiss;
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { notification: strims_notification_v1_INotification } ? { case: BodyCase.NOTIFICATION, notification: strims_notification_v1_Notification } :
    T extends { dismiss: bigint } ? { case: BodyCase.DISMISS, dismiss: bigint } :
    never
    >;
  };

}

export type IWatchRequest = Record<string, any>;

export class WatchRequest {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IWatchRequest) {
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
  event?: strims_notification_v1_IEvent;
}

export class WatchResponse {
  event: strims_notification_v1_Event | undefined;

  constructor(v?: IWatchResponse) {
    this.event = v?.event && new strims_notification_v1_Event(v.event);
  }

  static encode(m: WatchResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.event) strims_notification_v1_Event.encode(m.event, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): WatchResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new WatchResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.event = strims_notification_v1_Event.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
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
    const end = length === undefined ? r.len : r.pos + length;
    const m = new DismissRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        for (const flen = r.uint32(), fend = r.pos + flen; r.pos < fend;) m.ids.push(r.uint64());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IDismissResponse = Record<string, any>;

export class DismissResponse {

  // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
  constructor(v?: IDismissResponse) {
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

/* @internal */
export const strims_notification_v1_Notification = Notification;
/* @internal */
export type strims_notification_v1_Notification = Notification;
/* @internal */
export type strims_notification_v1_INotification = INotification;
/* @internal */
export const strims_notification_v1_Event = Event;
/* @internal */
export type strims_notification_v1_Event = Event;
/* @internal */
export type strims_notification_v1_IEvent = IEvent;
/* @internal */
export const strims_notification_v1_WatchRequest = WatchRequest;
/* @internal */
export type strims_notification_v1_WatchRequest = WatchRequest;
/* @internal */
export type strims_notification_v1_IWatchRequest = IWatchRequest;
/* @internal */
export const strims_notification_v1_WatchResponse = WatchResponse;
/* @internal */
export type strims_notification_v1_WatchResponse = WatchResponse;
/* @internal */
export type strims_notification_v1_IWatchResponse = IWatchResponse;
/* @internal */
export const strims_notification_v1_DismissRequest = DismissRequest;
/* @internal */
export type strims_notification_v1_DismissRequest = DismissRequest;
/* @internal */
export type strims_notification_v1_IDismissRequest = IDismissRequest;
/* @internal */
export const strims_notification_v1_DismissResponse = DismissResponse;
/* @internal */
export type strims_notification_v1_DismissResponse = DismissResponse;
/* @internal */
export type strims_notification_v1_IDismissResponse = IDismissResponse;
/* @internal */
export const strims_notification_v1_Notification_Subject = Notification.Subject;
/* @internal */
export type strims_notification_v1_Notification_Subject = Notification.Subject;
/* @internal */
export type strims_notification_v1_Notification_ISubject = Notification.ISubject;
/* @internal */
export const strims_notification_v1_Notification_Status = Notification.Status;
/* @internal */
export type strims_notification_v1_Notification_Status = Notification.Status;
/* @internal */
export const strims_notification_v1_Notification_Subject_Model = Notification.Subject.Model;
/* @internal */
export type strims_notification_v1_Notification_Subject_Model = Notification.Subject.Model;
