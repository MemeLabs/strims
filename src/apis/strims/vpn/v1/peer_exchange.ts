import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";


export type IPeerExchangeMessage = {
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
      case PeerExchangeMessage.BodyCase.MEDIATION_OFFER:
      PeerExchangeMessage.MediationOffer.encode(m.body.mediationOffer, w.uint32(8010).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.MEDIATION_ANSWER:
      PeerExchangeMessage.MediationAnswer.encode(m.body.mediationAnswer, w.uint32(8018).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.MEDIATION_ICE_CANDIDATE:
      PeerExchangeMessage.MediationIceCandidate.encode(m.body.mediationIceCandidate, w.uint32(8026).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.CALLBACK_REQUEST:
      PeerExchangeMessage.CallbackRequest.encode(m.body.callbackRequest, w.uint32(8034).fork()).ldelim();
      break;
      case PeerExchangeMessage.BodyCase.REJECTION:
      PeerExchangeMessage.Rejection.encode(m.body.rejection, w.uint32(8042).fork()).ldelim();
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
        case 1001:
        m.body = new PeerExchangeMessage.Body({ mediationOffer: PeerExchangeMessage.MediationOffer.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new PeerExchangeMessage.Body({ mediationAnswer: PeerExchangeMessage.MediationAnswer.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new PeerExchangeMessage.Body({ mediationIceCandidate: PeerExchangeMessage.MediationIceCandidate.decode(r, r.uint32()) });
        break;
        case 1004:
        m.body = new PeerExchangeMessage.Body({ callbackRequest: PeerExchangeMessage.CallbackRequest.decode(r, r.uint32()) });
        break;
        case 1005:
        m.body = new PeerExchangeMessage.Body({ rejection: PeerExchangeMessage.Rejection.decode(r, r.uint32()) });
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
    MEDIATION_OFFER = 1001,
    MEDIATION_ANSWER = 1002,
    MEDIATION_ICE_CANDIDATE = 1003,
    CALLBACK_REQUEST = 1004,
    REJECTION = 1005,
  }

  export type IBody =
  { case?: BodyCase.NOT_SET }
  |{ case?: BodyCase.MEDIATION_OFFER, mediationOffer: PeerExchangeMessage.IMediationOffer }
  |{ case?: BodyCase.MEDIATION_ANSWER, mediationAnswer: PeerExchangeMessage.IMediationAnswer }
  |{ case?: BodyCase.MEDIATION_ICE_CANDIDATE, mediationIceCandidate: PeerExchangeMessage.IMediationIceCandidate }
  |{ case?: BodyCase.CALLBACK_REQUEST, callbackRequest: PeerExchangeMessage.ICallbackRequest }
  |{ case?: BodyCase.REJECTION, rejection: PeerExchangeMessage.IRejection }
  ;

  export type TBody = Readonly<
  { case: BodyCase.NOT_SET }
  |{ case: BodyCase.MEDIATION_OFFER, mediationOffer: PeerExchangeMessage.MediationOffer }
  |{ case: BodyCase.MEDIATION_ANSWER, mediationAnswer: PeerExchangeMessage.MediationAnswer }
  |{ case: BodyCase.MEDIATION_ICE_CANDIDATE, mediationIceCandidate: PeerExchangeMessage.MediationIceCandidate }
  |{ case: BodyCase.CALLBACK_REQUEST, callbackRequest: PeerExchangeMessage.CallbackRequest }
  |{ case: BodyCase.REJECTION, rejection: PeerExchangeMessage.Rejection }
  >;

  class BodyImpl {
    mediationOffer: PeerExchangeMessage.MediationOffer;
    mediationAnswer: PeerExchangeMessage.MediationAnswer;
    mediationIceCandidate: PeerExchangeMessage.MediationIceCandidate;
    callbackRequest: PeerExchangeMessage.CallbackRequest;
    rejection: PeerExchangeMessage.Rejection;
    case: BodyCase = BodyCase.NOT_SET;

    constructor(v?: IBody) {
      if (v && "mediationOffer" in v) {
        this.case = BodyCase.MEDIATION_OFFER;
        this.mediationOffer = new PeerExchangeMessage.MediationOffer(v.mediationOffer);
      } else
      if (v && "mediationAnswer" in v) {
        this.case = BodyCase.MEDIATION_ANSWER;
        this.mediationAnswer = new PeerExchangeMessage.MediationAnswer(v.mediationAnswer);
      } else
      if (v && "mediationIceCandidate" in v) {
        this.case = BodyCase.MEDIATION_ICE_CANDIDATE;
        this.mediationIceCandidate = new PeerExchangeMessage.MediationIceCandidate(v.mediationIceCandidate);
      } else
      if (v && "callbackRequest" in v) {
        this.case = BodyCase.CALLBACK_REQUEST;
        this.callbackRequest = new PeerExchangeMessage.CallbackRequest(v.callbackRequest);
      } else
      if (v && "rejection" in v) {
        this.case = BodyCase.REJECTION;
        this.rejection = new PeerExchangeMessage.Rejection(v.rejection);
      }
    }
  }

  export const Body = BodyImpl as {
    new (): Readonly<{ case: BodyCase.NOT_SET }>;
    new <T extends IBody>(v: T): Readonly<
    T extends { mediationOffer: PeerExchangeMessage.IMediationOffer } ? { case: BodyCase.MEDIATION_OFFER, mediationOffer: PeerExchangeMessage.MediationOffer } :
    T extends { mediationAnswer: PeerExchangeMessage.IMediationAnswer } ? { case: BodyCase.MEDIATION_ANSWER, mediationAnswer: PeerExchangeMessage.MediationAnswer } :
    T extends { mediationIceCandidate: PeerExchangeMessage.IMediationIceCandidate } ? { case: BodyCase.MEDIATION_ICE_CANDIDATE, mediationIceCandidate: PeerExchangeMessage.MediationIceCandidate } :
    T extends { callbackRequest: PeerExchangeMessage.ICallbackRequest } ? { case: BodyCase.CALLBACK_REQUEST, callbackRequest: PeerExchangeMessage.CallbackRequest } :
    T extends { rejection: PeerExchangeMessage.IRejection } ? { case: BodyCase.REJECTION, rejection: PeerExchangeMessage.Rejection } :
    never
    >;
  };

  export type IMediationOffer = {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class MediationOffer {
    mediationId: bigint;
    data: Uint8Array;

    constructor(v?: IMediationOffer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: MediationOffer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data.length) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): MediationOffer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new MediationOffer();
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

  export type IMediationAnswer = {
    mediationId?: bigint;
    data?: Uint8Array;
  }

  export class MediationAnswer {
    mediationId: bigint;
    data: Uint8Array;

    constructor(v?: IMediationAnswer) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: MediationAnswer, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.data.length) w.uint32(18).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): MediationAnswer {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new MediationAnswer();
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

  export type IMediationIceCandidate = {
    mediationId?: bigint;
    index?: bigint;
    data?: Uint8Array;
  }

  export class MediationIceCandidate {
    mediationId: bigint;
    index: bigint;
    data: Uint8Array;

    constructor(v?: IMediationIceCandidate) {
      this.mediationId = v?.mediationId || BigInt(0);
      this.index = v?.index || BigInt(0);
      this.data = v?.data || new Uint8Array();
    }

    static encode(m: MediationIceCandidate, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      if (m.index) w.uint32(16).uint64(m.index);
      if (m.data.length) w.uint32(26).bytes(m.data);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): MediationIceCandidate {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new MediationIceCandidate();
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

  export type ICallbackRequest = Record<string, any>;

  export class CallbackRequest {

    // eslint-disable-next-line @typescript-eslint/no-unused-vars, @typescript-eslint/no-empty-function
    constructor(v?: ICallbackRequest) {
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

  export type IRejection = {
    mediationId?: bigint;
  }

  export class Rejection {
    mediationId: bigint;

    constructor(v?: IRejection) {
      this.mediationId = v?.mediationId || BigInt(0);
    }

    static encode(m: Rejection, w?: Writer): Writer {
      if (!w) w = new Writer();
      if (m.mediationId) w.uint32(8).uint64(m.mediationId);
      return w;
    }

    static decode(r: Reader | Uint8Array, length?: number): Rejection {
      r = r instanceof Reader ? r : new Reader(r);
      const end = length === undefined ? r.len : r.pos + length;
      const m = new Rejection();
      while (r.pos < end) {
        const tag = r.uint32();
        switch (tag >> 3) {
          case 1:
          m.mediationId = r.uint64();
          break;
          default:
          r.skipType(tag & 7);
          break;
        }
      }
      return m;
    }
  }

}

