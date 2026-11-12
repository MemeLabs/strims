// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React, { ReactNode, useEffect, useRef, useState } from "react";
import { Route } from "react-router-dom";

import { GetInvitationResponse } from "../../../apis/strims/network/v1/invite/invite";
import {
  UnimplementedInviteLinkService,
  registerInviteLinkService,
} from "../../../apis/strims/network/v1/invite/invite_rpc";
import {
  CreateNetworkFromInvitationRequest,
  CreateNetworkFromInvitationResponse,
  Invitation,
  InvitationV0,
  ListNetworksResponse,
} from "../../../apis/strims/network/v1/network";
import { UnimplementedNetworkFrontendService } from "../../../apis/strims/network/v1/network_rpc";
import InviteAuth from "../../../components/InviteAuth";
import NetworkService from "../../mocks/network/service";
import { MockApi, Story, StoryRouter, fixtures } from "../../storybook";
import { ProfileSession } from "./Layout/shell";

type InvitationAnswer = () => GetInvitationResponse;

// InviteAuth asks /api/invite for the invitation over HTTP (an InviteClient
// on an HTTPReadWriter, outside the FrontendClient): while mounted, the
// page's fetch answers that path from an InviteLink service in the page,
// whose getInvitation is `answer`, and passes other requests on.
const useInviteServer = (answer: InvitationAnswer): boolean => {
  const [ready, setReady] = useState(false);
  useEffect(() => {
    const registry = new ServiceRegistry();
    registerInviteLinkService(
      registry,
      new (class extends UnimplementedInviteLinkService {
        getInvitation() {
          return answer();
        }
      })()
    );
    const requests = new PassThrough();
    const responses = new PassThrough();
    new Host(responses, requests, registry);

    const fetch = window.fetch;
    window.fetch = (input, init) => {
      const url = input instanceof Request ? input.url : String(input);
      if (url !== "/api/invite" && url !== new URL("/api/invite", location.href).href) {
        return fetch(input, init);
      }
      return new Promise<Response>((resolve) => {
        responses.once("data", (data: Uint8Array) => resolve(new Response(new Uint8Array(data))));
        requests.write(init.body as Uint8Array);
      });
    };
    setReady(true);
    return () => {
      window.fetch = fetch;
      requests.destroy();
      responses.destroy();
    };
  }, []);
  return ready;
};

// An invitation from jbpratt to network `i` (network 1..3 are the
// profile's already), naming `bootstrap` servers.
const invitation =
  (i: number, bootstrap = true): InvitationAnswer =>
  () => {
    const network = fixtures.network(i, { alias: "jbpratt", owner: false });
    const data = InvitationV0.decode(fixtures.invitation(network).data);
    if (!bootstrap) {
      data.bootstrapClients = [];
    }
    return new GetInvitationResponse({
      invitation: new Invitation({ version: 0, data: InvitationV0.encode(data).finish() }),
    });
  };

const refused = (): InvitationAnswer => () => {
  throw new Error("invitation not found");
};

interface NetworkOptions {
  listDelayMs?: number;
  joinError?: string;
}

// The profile's networks (fixtures 1..3), listed after `listDelayMs`;
// joining adds one, or is refused with `joinError`.
class InviteNetworkService extends UnimplementedNetworkFrontendService {
  private network = new NetworkService({ count: 3 });

  constructor(private opts: NetworkOptions = {}) {
    super();
  }

  list(): ListNetworksResponse | Promise<ListNetworksResponse> {
    const { listDelayMs } = this.opts;
    if (!listDelayMs) {
      return this.network.list();
    }
    return new Promise((resolve) => setTimeout(() => resolve(this.network.list()), listDelayMs));
  }

  createNetworkFromInvitation(
    req: CreateNetworkFromInvitationRequest
  ): CreateNetworkFromInvitationResponse {
    if (this.opts.joinError) {
      throw new Error(this.opts.joinError);
    }
    return this.network.createNetworkFromInvitation(req);
  }
}

// Types into the form's alias field and submits the form, as a viewer would.
const useSubmit = (root: React.RefObject<HTMLElement>, alias?: string) =>
  useEffect(() => {
    if (alias === undefined) {
      return;
    }
    let frame = 0;
    const submit = () => {
      const form = root.current?.querySelector("form");
      if (!form) {
        frame = requestAnimationFrame(submit);
        return;
      }
      const input = form.querySelector<HTMLInputElement>("input[name=alias]");
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value").set.call(input, alias);
      input.dispatchEvent(new Event("input", { bubbles: true }));
      form.requestSubmit();
    };
    frame = requestAnimationFrame(submit);
    return () => cancelAnimationFrame(frame);
  }, []);

interface InviteStoryProps {
  answer: InvitationAnswer;
  network?: NetworkOptions;
  /** Submits the form with this alias. */
  submitAlias?: string;
}

const Ready: React.FC<{ answer: InvitationAnswer; children: ReactNode }> = ({
  answer,
  children,
}) => (useInviteServer(answer) ? <>{children}</> : null);

// The invite page's modal over the page; accepting or rejecting the
// invitation navigates inside the story.
const InviteStory: React.FC<InviteStoryProps> = ({ answer, network, submitAlias }) => {
  const ref = useRef<HTMLDivElement>(null);
  useSubmit(ref, submitAlias);
  return (
    <MockApi services={() => ({ network: new InviteNetworkService(network) })}>
      <ProfileSession>
        <Ready answer={answer}>
          <StoryRouter initialPath="/invite/abc123">
            <Route
              path="/invite/:code"
              element={
                <div ref={ref}>
                  <InviteAuth code="abc123" />
                </div>
              }
            />
          </StoryRouter>
        </Ready>
      </ProfileSession>
    </MockApi>
  );
};

const stories: Story[] = [
  {
    name: "Invitation",
    component: () => <InviteStory answer={invitation(7)} />,
  },
  {
    name: "No bootstrap servers",
    component: () => <InviteStory answer={invitation(7, false)} />,
  },
  {
    name: "Loading",
    component: () => <InviteStory answer={invitation(7)} network={{ listDelayMs: 4000 }} />,
  },
  {
    name: "Error",
    component: () => <InviteStory answer={refused()} />,
  },
  {
    name: "Invalid alias",
    component: () => <InviteStory answer={invitation(7)} submitAlias="not a valid name!" />,
  },
  {
    name: "Join error",
    component: () => (
      <InviteStory
        answer={invitation(7)}
        network={{ joinError: "the network's bootstrap servers can't be reached" }}
        submitAlias="test_user"
      />
    ),
  },
  {
    name: "Already a member",
    component: () => <InviteStory answer={invitation(2)} />,
  },
];

export default stories;
