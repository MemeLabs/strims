// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route } from "react-router-dom";

import { GetInvitationResponse } from "../../../apis/strims/network/v1/invite/invite";
import { UnimplementedInviteLinkService } from "../../../apis/strims/network/v1/invite/invite_rpc";
import { Network } from "../../../apis/strims/network/v1/network";
import Invite from "../../../pages/Invite";
import NetworkService from "../../mocks/network/service";
import { MockApi, MockServices, Story, StoryRouter, fixtures } from "../../storybook";
import { FetchStub, inviteRoute } from "./fetchStub";
import { StorySession } from "./StorySession";

// The network the invitation is to, by its peer jbpratt.
const invitedNetwork = (): Network => fixtures.network(6, { alias: "jbpratt", owner: false });

// The invite API's answer: an invitation to `network`, or this error.
class InviteLinkService extends UnimplementedInviteLinkService {
  constructor(private readonly answer: Network | Error) {
    super();
  }

  getInvitation() {
    if (this.answer instanceof Error) {
      throw this.answer;
    }
    return new GetInvitationResponse({ invitation: fixtures.invitation(this.answer) });
  }
}

interface InviteStoryProps {
  /** The node's network frontend; default networks 1..3, without the invited one. */
  network?: () => MockServices["network"];
  invitation?: Network | Error;
}

// The page at /invite/:code: a modal in the layout's root over the
// invitation the invite API (an HTTP POST, stubbed) gives for the code.
// Joining or rejecting navigates in the story.
const InviteStory: React.FC<InviteStoryProps> = ({
  network = () => new NetworkService({ count: 3 }),
  invitation = invitedNetwork(),
}) => (
  <FetchStub route={inviteRoute(new InviteLinkService(invitation))}>
    <MockApi services={() => ({ network: network() })}>
      <StorySession state={{ profile: fixtures.profile() }}>
        <StoryRouter initialPath="/invite/bhPfc3a9">
          <Route path="/invite/:code" element={<Invite />} />
        </StoryRouter>
      </StorySession>
    </MockApi>
  </FetchStub>
);

// Networks 1..3 whose list answers after 4 s.
const slowNetworks = (): MockServices["network"] => {
  const service = new NetworkService({ count: 3 });
  return {
    list: () => new Promise((resolve) => setTimeout(() => resolve(service.list()), 4000)),
    createNetworkFromInvitation: (req) => service.createNetworkFromInvitation(req),
  };
};

const stories: Story[] = [
  {
    // An invitation to a network the node isn't in.
    name: "Default",
    component: () => <InviteStory />,
  },
  {
    // The node's networks take 4 s to list.
    name: "Loading",
    component: () => <InviteStory network={slowNetworks} />,
  },
  {
    // The invite API refused the code.
    name: "Error",
    component: () => <InviteStory invitation={new Error("invitation not found")} />,
  },
  {
    // An invitation to network 2, which the node is in: on to its directory.
    name: "Member",
    component: () => <InviteStory invitation={fixtures.network(2, { alias: "jbpratt" })} />,
  },
];

export default stories;
