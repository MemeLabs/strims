// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React, { ReactNode, useLayoutEffect } from "react";

import {
  InviteLinkService,
  registerInviteLinkService,
} from "../../../apis/strims/network/v1/invite/invite_rpc";

/** An answer for a fetch the stub takes, or null to let it through. */
export type FetchRoute = (url: URL, init?: RequestInit) => Promise<Response> | null;

// Answers the page's fetches `route` takes until the returned function puts
// fetch back.
export const stubFetch = (route: FetchRoute): (() => void) => {
  const original = window.fetch;
  const through = original.bind(window) as typeof fetch;
  window.fetch = (input, init) => {
    const href = input instanceof Request ? input.url : String(input);
    return route(new URL(href, window.location.href), init) ?? through(input, init);
  };
  return () => {
    window.fetch = original;
  };
};

export interface FetchStubProps {
  /** Read once, at mount. */
  route: FetchRoute;
  children: ReactNode;
}

// Stubs fetch while mounted. A layout effect: it runs before any effect
// below it fetches.
export const FetchStub: React.FC<FetchStubProps> = ({ route, children }) => {
  useLayoutEffect(() => stubFetch(route), []);
  return <>{children}</>;
};

// The invite API's route (POST /api/invite): each request's body is an RPC
// call, which `service` answers in the response's body, as lib/http's
// HTTPReadWriter expects.
export const inviteRoute = (service: InviteLinkService): FetchRoute => {
  const registry = new ServiceRegistry();
  registerInviteLinkService(registry, service);
  return (url, init) => {
    if (url.pathname !== "/api/invite") {
      return null;
    }
    const req = new PassThrough({ objectMode: true });
    const res = new PassThrough({ objectMode: true });
    new Host(res, req, registry);
    const answer = new Promise<Response>((resolve) =>
      res.once("data", (data: Uint8Array) =>
        resolve(
          new Response(new Uint8Array(data), {
            headers: { "Content-Type": "application/protobuf" },
          })
        )
      )
    );
    req.write(new Uint8Array(init.body as Uint8Array));
    return answer;
  };
};
