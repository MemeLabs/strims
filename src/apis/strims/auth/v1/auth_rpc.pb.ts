// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { default as strims_rpc_Host, UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import type strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import type { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc.pb";

import {
  type ISignInRequest as strims_auth_v1_ISignInRequest,
  SignInRequest as strims_auth_v1_SignInRequest,
  SignInResponse as strims_auth_v1_SignInResponse,
  type ISignUpRequest as strims_auth_v1_ISignUpRequest,
  SignUpRequest as strims_auth_v1_SignUpRequest,
  SignUpResponse as strims_auth_v1_SignUpResponse,
} from "./auth.pb";

export interface AuthFrontendService {
  signIn(req: strims_auth_v1_SignInRequest, call: strims_rpc_Call): Promise<strims_auth_v1_SignInResponse> | strims_auth_v1_SignInResponse;
  signUp(req: strims_auth_v1_SignUpRequest, call: strims_rpc_Call): Promise<strims_auth_v1_SignUpResponse> | strims_auth_v1_SignUpResponse;
}

export class UnimplementedAuthFrontendService implements AuthFrontendService {
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  signIn(req: strims_auth_v1_SignInRequest, call: strims_rpc_Call): Promise<strims_auth_v1_SignInResponse> | strims_auth_v1_SignInResponse { throw new Error("not implemented"); }
  // eslint-disable-next-line @typescript-eslint/no-unused-vars
  signUp(req: strims_auth_v1_SignUpRequest, call: strims_rpc_Call): Promise<strims_auth_v1_SignUpResponse> | strims_auth_v1_SignUpResponse { throw new Error("not implemented"); }
}

export const registerAuthFrontendService = (host: strims_rpc_Service, service: AuthFrontendService): void => {
  host.registerMethod<strims_auth_v1_SignInRequest, strims_auth_v1_SignInResponse>("strims.auth.v1.AuthFrontend.SignIn", (req, call) => service.signIn(req, call), strims_auth_v1_SignInRequest);
  host.registerMethod<strims_auth_v1_SignUpRequest, strims_auth_v1_SignUpResponse>("strims.auth.v1.AuthFrontend.SignUp", (req, call) => service.signUp(req, call), strims_auth_v1_SignUpRequest);
}

export class AuthFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public signIn(req?: strims_auth_v1_ISignInRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_auth_v1_SignInResponse> {
    return this.host.expectOne(this.host.call("strims.auth.v1.AuthFrontend.SignIn", new strims_auth_v1_SignInRequest(req)), strims_auth_v1_SignInResponse, opts);
  }

  public signUp(req?: strims_auth_v1_ISignUpRequest, opts?: strims_rpc_UnaryCallOptions): Promise<strims_auth_v1_SignUpResponse> {
    return this.host.expectOne(this.host.call("strims.auth.v1.AuthFrontend.SignUp", new strims_auth_v1_SignUpRequest(req)), strims_auth_v1_SignUpResponse, opts);
  }
}

