// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  LinkDescription as strims_vnic_v1_LinkDescription,
  type ILinkDescription as strims_vnic_v1_ILinkDescription,
} from "../../vnic/v1/vnic.pb";

export type IPeerExchangeMessage = {
  body?: PeerExchangeMessage_IBody
}

export class PeerExchangeMessage {
  body: PeerExchangeMessage_TBody;

  constructor(v?: IPeerExchangeMessage) {
    this.body = v?.body ? new PeerExchangeMessage_Body(v.body) : new PeerExchangeMessage_Body();
  }

  static encode(m: PeerExchangeMessage, w?: Writer): Writer {
    if (!w) w = new Writer();
    switch (m.body.case) {
      case PeerExchangeMessage_BodyCase.LINK_OFFER:
      PeerExchangeMessage_LinkOffer.encode(m.body.linkOffer, w.uint32(8010).fork()).ldelim();
      break;
      case PeerExchangeMessage_BodyCase.LINK_ANSWER:
      PeerExchangeMessage_LinkAnswer.encode(m.body.linkAnswer, w.uint32(8018).fork()).ldelim();
      break;
      case PeerExchangeMessage_BodyCase.REJECTION:
      PeerExchangeMessage_Rejection.encode(m.body.rejection, w.uint32(8026).fork()).ldelim();
      break;
    }
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerExchangeMessage {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerExchangeMessage();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1001:
        m.body = new PeerExchangeMessage_Body({ linkOffer: PeerExchangeMessage_LinkOffer.decode(r, r.uint32()) });
        break;
        case 1002:
        m.body = new PeerExchangeMessage_Body({ linkAnswer: PeerExchangeMessage_LinkAnswer.decode(r, r.uint32()) });
        break;
        case 1003:
        m.body = new PeerExchangeMessage_Body({ rejection: PeerExchangeMessage_Rejection.decode(r, r.uint32()) });
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

export enum PeerExchangeMessage_BodyCase {
  NOT_SET = 0,
  LINK_OFFER = 1001,
  LINK_ANSWER = 1002,
  REJECTION = 1003,
}

export type PeerExchangeMessage_IBody =
{ case?: PeerExchangeMessage_BodyCase.NOT_SET }
|{ case?: PeerExchangeMessage_BodyCase.LINK_OFFER, linkOffer: PeerExchangeMessage_ILinkOffer }
|{ case?: PeerExchangeMessage_BodyCase.LINK_ANSWER, linkAnswer: PeerExchangeMessage_ILinkAnswer }
|{ case?: PeerExchangeMessage_BodyCase.REJECTION, rejection: PeerExchangeMessage_IRejection }
;

export type PeerExchangeMessage_TBody = Readonly<
{ case: PeerExchangeMessage_BodyCase.NOT_SET }
|{ case: PeerExchangeMessage_BodyCase.LINK_OFFER, linkOffer: PeerExchangeMessage_LinkOffer }
|{ case: PeerExchangeMessage_BodyCase.LINK_ANSWER, linkAnswer: PeerExchangeMessage_LinkAnswer }
|{ case: PeerExchangeMessage_BodyCase.REJECTION, rejection: PeerExchangeMessage_Rejection }
>;

class PeerExchangeMessage_BodyImpl {
  declare linkOffer?: PeerExchangeMessage_LinkOffer;
  declare linkAnswer?: PeerExchangeMessage_LinkAnswer;
  declare rejection?: PeerExchangeMessage_Rejection;
  case: PeerExchangeMessage_BodyCase = PeerExchangeMessage_BodyCase.NOT_SET;

  constructor(v?: PeerExchangeMessage_IBody) {
    if (v && "linkOffer" in v) {
      this.case = PeerExchangeMessage_BodyCase.LINK_OFFER;
      this.linkOffer = new PeerExchangeMessage_LinkOffer(v.linkOffer);
    } else
    if (v && "linkAnswer" in v) {
      this.case = PeerExchangeMessage_BodyCase.LINK_ANSWER;
      this.linkAnswer = new PeerExchangeMessage_LinkAnswer(v.linkAnswer);
    } else
    if (v && "rejection" in v) {
      this.case = PeerExchangeMessage_BodyCase.REJECTION;
      this.rejection = new PeerExchangeMessage_Rejection(v.rejection);
    }
  }
}

export const PeerExchangeMessage_Body = PeerExchangeMessage_BodyImpl as {
  new (): Readonly<{ case: PeerExchangeMessage_BodyCase.NOT_SET }>;
  new <T extends PeerExchangeMessage_IBody>(v: T): Readonly<
  T extends { linkOffer: PeerExchangeMessage_ILinkOffer } ? { case: PeerExchangeMessage_BodyCase.LINK_OFFER, linkOffer: PeerExchangeMessage_LinkOffer } :
  T extends { linkAnswer: PeerExchangeMessage_ILinkAnswer } ? { case: PeerExchangeMessage_BodyCase.LINK_ANSWER, linkAnswer: PeerExchangeMessage_LinkAnswer } :
  T extends { rejection: PeerExchangeMessage_IRejection } ? { case: PeerExchangeMessage_BodyCase.REJECTION, rejection: PeerExchangeMessage_Rejection } :
  never
  >;
};

export type PeerExchangeMessage_ILinkOffer = {
  exchangeId?: bigint;
  descriptions?: strims_vnic_v1_ILinkDescription[];
}

export class PeerExchangeMessage_LinkOffer {
  exchangeId: bigint;
  descriptions: strims_vnic_v1_LinkDescription[];

  constructor(v?: PeerExchangeMessage_ILinkOffer) {
    this.exchangeId = v?.exchangeId || BigInt(0);
    this.descriptions = v?.descriptions ? v.descriptions.map(v => new strims_vnic_v1_LinkDescription(v)) : [];
  }

  static encode(m: PeerExchangeMessage_LinkOffer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.exchangeId) w.uint32(8).uint64(m.exchangeId);
    for (const v of m.descriptions) strims_vnic_v1_LinkDescription.encode(v, w.uint32(18).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerExchangeMessage_LinkOffer {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerExchangeMessage_LinkOffer();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.exchangeId = r.uint64();
        break;
        case 2:
        m.descriptions.push(strims_vnic_v1_LinkDescription.decode(r, r.uint32()));
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

export type PeerExchangeMessage_ILinkAnswer = {
  exchangeId?: bigint;
  descriptions?: strims_vnic_v1_ILinkDescription[];
  errorMessage?: string;
}

export class PeerExchangeMessage_LinkAnswer {
  exchangeId: bigint;
  descriptions: strims_vnic_v1_LinkDescription[];
  errorMessage: string;

  constructor(v?: PeerExchangeMessage_ILinkAnswer) {
    this.exchangeId = v?.exchangeId || BigInt(0);
    this.descriptions = v?.descriptions ? v.descriptions.map(v => new strims_vnic_v1_LinkDescription(v)) : [];
    this.errorMessage = v?.errorMessage || "";
  }

  static encode(m: PeerExchangeMessage_LinkAnswer, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.exchangeId) w.uint32(8).uint64(m.exchangeId);
    for (const v of m.descriptions) strims_vnic_v1_LinkDescription.encode(v, w.uint32(18).fork()).ldelim();
    if (m.errorMessage.length) w.uint32(26).string(m.errorMessage);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerExchangeMessage_LinkAnswer {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerExchangeMessage_LinkAnswer();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.exchangeId = r.uint64();
        break;
        case 2:
        m.descriptions.push(strims_vnic_v1_LinkDescription.decode(r, r.uint32()));
        break;
        case 3:
        m.errorMessage = r.string();
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

export type PeerExchangeMessage_IRejection = {
  mediationId?: bigint;
}

export class PeerExchangeMessage_Rejection {
  mediationId: bigint;

  constructor(v?: PeerExchangeMessage_IRejection) {
    this.mediationId = v?.mediationId || BigInt(0);
  }

  static encode(m: PeerExchangeMessage_Rejection, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.mediationId) w.uint32(8).uint64(m.mediationId);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): PeerExchangeMessage_Rejection {
    r = r instanceof Reader ? r : new Reader(r);
    const parentEnd = r.enter(length);
    const end = r.end;
    const m = new PeerExchangeMessage_Rejection();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.mediationId = r.uint64();
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

