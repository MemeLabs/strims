// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type IGetInvitationRequest as strims_network_v1_invite_IGetInvitationRequest,
  GetInvitationRequest as strims_network_v1_invite_GetInvitationRequest,
  GetInvitationResponse as strims_network_v1_invite_GetInvitationResponse,
} from "./invite.pb";

export interface InviteLinkService {
  getInvitation(req: strims_network_v1_invite_GetInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_invite_GetInvitationResponse> | strims_network_v1_invite_GetInvitationResponse;
}

export class UnimplementedInviteLinkService implements InviteLinkService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  getInvitation(req: strims_network_v1_invite_GetInvitationRequest, call: strims_rpc_Call): Promise<strims_network_v1_invite_GetInvitationResponse> | strims_network_v1_invite_GetInvitationResponse { throw new Error("not implemented"); }
}

export const registerInviteLinkService = (host: strims_rpc_Service, service: InviteLinkService): void => {
  host.registerMethod<strims_network_v1_invite_GetInvitationRequest, strims_network_v1_invite_GetInvitationResponse>("strims.network.v1.invite.InviteLink.GetInvitation", (req, call) => service.getInvitation(req, call), strims_network_v1_invite_GetInvitationRequest);
}

export class InviteLinkClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public getInvitation(req?: strims_network_v1_invite_IGetInvitationRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_network_v1_invite_GetInvitationResponse> {
    return this.host.expectOne(this.host.call("strims.network.v1.invite.InviteLink.GetInvitation", new strims_network_v1_invite_GetInvitationRequest(req)), strims_network_v1_invite_GetInvitationResponse, opts);
  }
}

