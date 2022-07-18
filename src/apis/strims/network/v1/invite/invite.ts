import Reader from "@memelabs/protobuf/lib/pb/reader";
import Writer from "@memelabs/protobuf/lib/pb/writer";

import {
  strims_network_v1_Invitation,
  strims_network_v1_IInvitation,
} from "../network";

export type IGetInvitationRequest = {
  code?: string;
}

export class GetInvitationRequest {
  code: string;

  constructor(v?: IGetInvitationRequest) {
    this.code = v?.code || "";
  }

  static encode(m: GetInvitationRequest, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.code.length) w.uint32(10).string(m.code);
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetInvitationRequest {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetInvitationRequest();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.code = r.string();
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

export type IGetInvitationResponse = {
  invitation?: strims_network_v1_IInvitation;
}

export class GetInvitationResponse {
  invitation: strims_network_v1_Invitation | undefined;

  constructor(v?: IGetInvitationResponse) {
    this.invitation = v?.invitation && new strims_network_v1_Invitation(v.invitation);
  }

  static encode(m: GetInvitationResponse, w?: Writer): Writer {
    if (!w) w = new Writer();
    if (m.invitation) strims_network_v1_Invitation.encode(m.invitation, w.uint32(10).fork()).ldelim();
    return w;
  }

  static decode(r: Reader | Uint8Array, length?: number): GetInvitationResponse {
    r = r instanceof Reader ? r : new Reader(r);
    const end = length === undefined ? r.len : r.pos + length;
    const m = new GetInvitationResponse();
    while (r.pos < end) {
      const tag = r.uint32();
      switch (tag >> 3) {
        case 1:
        m.invitation = strims_network_v1_Invitation.decode(r, r.uint32());
        break;
        default:
        r.skipType(tag & 7);
        break;
      }
    }
    return m;
  }
}

/* @internal */
export const strims_network_v1_invite_GetInvitationRequest = GetInvitationRequest;
/* @internal */
export type strims_network_v1_invite_GetInvitationRequest = GetInvitationRequest;
/* @internal */
export type strims_network_v1_invite_IGetInvitationRequest = IGetInvitationRequest;
/* @internal */
export const strims_network_v1_invite_GetInvitationResponse = GetInvitationResponse;
/* @internal */
export type strims_network_v1_invite_GetInvitationResponse = GetInvitationResponse;
/* @internal */
export type strims_network_v1_invite_IGetInvitationResponse = IGetInvitationResponse;
