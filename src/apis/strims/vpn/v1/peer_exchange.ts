import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export interface IPeerExchangeMessage {
  body?: PeerExchangeMessage.IBody
}

export class PeerExchangeMessage {
  body: PeerExchangeMessage.TBody;

  constructor(v?: IPeerExchangeMessage) {
    this.body = new PeerExchangeMessage.Body(v?.body);
  }

  static encode(m: PeerExchangeMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case PeerExchangeMessage.BodyCase.REQUEST:
      PeerExchangeMessage.Request.encode(m.body.request, w.uint32(10).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.RESPONSE:
      PeerExchangeMessage.Response.encode(m.body.response, w.uint32(18).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.OFFER:
      PeerExchangeMessage.Offer.encode(m.body.offer, w.uint32(26).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.ANSWER:
      PeerExchangeMessage.Answer.encode(m.body.answer, w.uint32(34).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.ICE_CANDIDATE:
      PeerExchangeMessage.IceCandidate.encode(m.body.iceCandidate, w.uint32(42).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.CALLBACK_REQUEST:
      PeerExchangeMessage.CallbackRequest.encode(m.body.callbackRequest, w.uint32(50).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerExchangeMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new PeerExchangeMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.body = new PeerExchangeMessage.Body({ request: PeerExchangeMessage.Request.decode(r, r.uint32()) });
        break;
        case 2:
        m.body = new PeerExchangeMessage.Body({ response: PeerExchangeMessage.Response.decode(r, r.uint32()) });
        break;
        case 3:
        m.body = new PeerExchangeMessage.Body({ offer: PeerExchangeMessage.Offer.decode(r, r.uint32()) });
        break;
        case 4:
        m.body = new PeerExchangeMessage.Body({ answer: PeerExchangeMessage.Answer.decode(r, r.uint32()) });
        break;
        case 5:
        m.body = new PeerExchangeMessage.Body({ iceCandidate: PeerExchangeMessage.IceCandidate.decode(r, r.uint32()) });
        break;
        case 6:
        m.body = new PeerExchangeMessage.Body({ callbackRequest: PeerExchangeMessage.CallbackRequest.decode(r, r.uint32()) });
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export namespace PeerExchangeMessage {
  export enum BodyCase {
    NOT_SET = 0,
    REQUEST = 1,
    RESPONSE = 2,
    OFFER = 3,
    ANSWER = 4,
    ICE_CANDIDATE = 5,
    CALLBACK_REQUEST = 6,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.REQUEST, request: PeerExchangeMessage.IRequest }
  |{ case?: BodyCase.RESPONSE, response: PeerExchangeMessage.IResponse }
  |{ case?: BodyCase.OFFER, offer: PeerExchangeMessage.IOffer }
  |{ case?: BodyCase.ANSWER, answer: PeerExchangeMessage.IAnswer }
  |{ case?: BodyCase.ICE_CANDIDATE, iceCandidate: PeerExchangeMessage.IIceCandidate }
  |{ case?: BodyCase.CALLBACK_REQUEST, callbackRequest: PeerExchangeMessage.ICallbackRequest }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.REQUEST, request: PeerExchangeMessage.Request }
  |{ case: BodyCase.RESPONSE, response: PeerExchangeMessage.Response }
  |{ case: BodyCase.OFFER, offer: PeerExchangeMessage.Offer }
  |{ case: BodyCase.ANSWER, answer: PeerExchangeMessage.Answer }
  |{ case: BodyCase.ICE_CANDIDATE, iceCandidate: PeerExchangeMessage.IceCandidate }
  |{ case: BodyCase.CALLBACK_REQUEST, callbackRequest: PeerExchangeMessage.CallbackRequest }
  >;

  class BodyImpl {
    request: PeerExchangeMessage.Request;
    response: PeerExchangeMessage.Response;
    offer: PeerExchangeMessage.Offer;
    answer: PeerExchangeMessage.Answer;
    iceCandidate: PeerExchangeMessage.IceCandidate;
    callbackRequest: PeerExchangeMessage.CallbackRequest;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "request" in v) {
        this.case = BodyCase.REQUEST;
        this.request = new PeerExchangeMessage.Request(v.request);
      } else
      if (v && "response" in v) {
        this.case = BodyCase.RESPONSE;
        this.response = new PeerExchangeMessage.Response(v.response);
      } else
      if (v && "offer" in v) {
        this.case = BodyCase.OFFER;
        this.offer = new PeerExchangeMessage.Offer(v.offer);
      } else
      if (v && "answer" in v) {
        this.case = BodyCase.ANSWER;
        this.answer = new PeerExchangeMessage.Answer(v.answer);
      } else
      if (v && "iceCandidate" in v) {
        this.case = BodyCase.ICE_CANDIDATE;
        this.iceCandidate = new PeerExchangeMessage.IceCandidate(v.iceCandidate);
      } else
      if (v && "callbackRequest" in v) {
        this.case = BodyCase.CALLBACK_REQUEST;
        this.callbackRequest = new PeerExchangeMessage.CallbackRequest(v.callbackRequest);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { request: PeerExchangeMessage.IRequest } ? { case: BodyCase.REQUEST, request: PeerExchangeMessage.Request } :
    T extends { response: PeerExchangeMessage.IResponse } ? { case: BodyCase.RESPONSE, response: PeerExchangeMessage.Response } :
    T extends { offer: PeerExchangeMessage.IOffer } ? { case: BodyCase.OFFER, offer: PeerExchangeMessage.Offer } :
    T extends { answer: PeerExchangeMessage.IAnswer } ? { case: BodyCase.ANSWER, answer: PeerExchangeMessage.Answer } :
    T extends { iceCandidate: PeerExchangeMessage.IIceCandidate } ? { case: BodyCase.ICE_CANDIDATE, iceCandidate: PeerExchangeMessage.IceCandidate } :
    T extends { callbackRequest: PeerExchangeMessage.ICallbackRequest } ? { case: BodyCase.CALLBACK_REQUEST, callbackRequest: PeerExchangeMessage.CallbackRequest } :
    never
    >;
  };

  export interface IRequest {
    count?: number;
  }

  export class Request {
    count: number = 0;

    constructor(v?: IRequest) {
      this.count = v?.count || 0;
    }

    static encode(m: Request, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.count) w.uint32(8).uint32(m.count);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Request {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Request();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.count = r.uint32();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IResponse {
    ids?: Uint8Array[];
  }

  export class Response {
    ids: Uint8Array[] = [];

    constructor(v?: IResponse) {
      if (v?.ids) this.ids = v.ids;
    }

    static encode(m: Response, w?: Writer): Writer {
      if (!w) w = new Writer();
      for (const v of m.ids) w.uint32(10).bytes(v);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Response {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Response();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.ids.push(r.bytes())
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IOffer {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class Offer {
    mediationId: bigint = BigInt(0);
    data: Uint8Array = new Uint8Array();

    constructor(v?: IOffer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: Offer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Offer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Offer();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
          break;
          case 2:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IAnswer {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class Answer {
    mediationId: bigint = BigInt(0);
    data: Uint8Array = new Uint8Array();

    constructor(v?: IAnswer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: Answer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Answer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Answer();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
          break;
          case 2:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface IIceCandidate {
    mediationId?: bigint;
    index?: bigint;
    data?: Uint8Array;
  }

  export class IceCandidate {
    mediationId: bigint = BigInt(0);
    index: bigint = BigInt(0);
    data: Uint8Array = new Uint8Array();

    constructor(v?: IIceCandidate) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.index = v?.index || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: IceCandidate, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.index) w.uint32(16).uint64(m.index);
      if (m.data) w.uint32(26).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): IceCandidate {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new IceCandidate();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
          break;
          case 2:
          m.index = r.uint64();
          break;
          case 3:
          m.data = r.bytes();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

  export interface ICallbackRequest {
  }

  export class CallbackRequest {

    constructor(v?: ICallbackRequest) {
      // noop
    }

    static encode(m: CallbackRequest, w?: Writer): Writer {
      if (!w) w = new Writer();
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): CallbackRequest {
      if (r instanceof Reader && length) r.skip(length);
      return new CallbackRequest();
    }
  }

}

