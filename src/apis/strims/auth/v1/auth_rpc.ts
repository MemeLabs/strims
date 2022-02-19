import strims_rpc_Host, { UnaryCallOptions as strims_rpc_UnaryCallOptions } from "@memelabs/protobuf/lib/rpc/host";
import strims_rpc_Service from "@memelabs/protobuf/lib/rpc/service";
import { Call as strims_rpc_Call } from "@memelabs/protobuf/lib/apis/strims/rpc/rpc";

import {
  ISignInRequest,
  SignInRequest,
  SignInResponse,
  ISignUpRequest,
  SignUpRequest,
  SignUpResponse,
} from "./auth";

export interface AuthFrontendService {
  signIn(req: SignInRequest, call: strims_rpc_Call): Promise<SignInResponse> | SignInResponse;
  signUp(req: SignUpRequest, call: strims_rpc_Call): Promise<SignUpResponse> | SignUpResponse;
}

export class UnimplementedAuthFrontendService implements AuthFrontendService {
  signIn(req: SignInRequest, call: strims_rpc_Call): Promise<SignInResponse> | SignInResponse { throw new Error("not implemented"); }
  signUp(req: SignUpRequest, call: strims_rpc_Call): Promise<SignUpResponse> | SignUpResponse { throw new Error("not implemented"); }
}

export const registerAuthFrontendService = (host: strims_rpc_Service, service: AuthFrontendService): void => {
  host.registerMethod<SignInRequest, SignInResponse>("strims.auth.v1.AuthFrontend.SignIn", service.signIn.bind(service), SignInRequest);
  host.registerMethod<SignUpRequest, SignUpResponse>("strims.auth.v1.AuthFrontend.SignUp", service.signUp.bind(service), SignUpRequest);
}

export class AuthFrontendClient {
  constructor(private readonly host: strims_rpc_Host) {}

  public signIn(req?: ISignInRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SignInResponse> {
    return this.host.expectOne(this.host.call("strims.auth.v1.AuthFrontend.SignIn", new SignInRequest(req)), SignInResponse, opts);
  }

  public signUp(req?: ISignUpRequest, opts?: strims_rpc_UnaryCallOptions): Promise<SignUpResponse> {
    return this.host.expectOne(this.host.call("strims.auth.v1.AuthFrontend.SignUp", new SignUpRequest(req)), SignUpResponse, opts);
  }
}

