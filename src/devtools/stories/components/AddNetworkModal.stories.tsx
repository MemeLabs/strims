// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useRef, useState } from "react";

import { CreateServerRequest, CreateServerResponse } from "../../../apis/strims/network/v1/network";
import { UnimplementedNetworkFrontendService } from "../../../apis/strims/network/v1/network_rpc";
import AddNetworkModal from "../../../components/AddNetworkModal";
import NetworkService from "../../mocks/network/service";
import { MockApi, Story } from "../../storybook";
import { ProfileSession } from "./Layout/shell";

// createServer as the network mock answers it, after `delayMs`, or refused.
// (A refusal throws: the RPC host answers a rejected promise as an error but
// leaves a rejection of its own unhandled.)
class CreateServerService extends UnimplementedNetworkFrontendService {
  private network = new NetworkService({ count: 3 });

  constructor(private opts: { delayMs?: number; refuse?: string }) {
    super();
  }

  createServer(req: CreateServerRequest): Promise<CreateServerResponse> {
    const { delayMs = 0, refuse } = this.opts;
    if (refuse) {
      throw new Error(refuse);
    }
    return new Promise((resolve) =>
      setTimeout(() => resolve(this.network.createServer(req)), delayMs)
    );
  }
}

interface ModalStoryProps {
  network?: { delayMs?: number; refuse?: string };
  signedIn?: boolean;
  /** Submits the form on mount. */
  submit?: boolean;
}

// The modal over the page; creating or closing it says so in its place.
const ModalStory: React.FC<ModalStoryProps> = ({ network = {}, signedIn = true, submit }) => {
  const ref = useRef<HTMLDivElement>(null);
  const [done, setDone] = useState<string>(null);
  useEffect(() => {
    if (submit) {
      ref.current.querySelector("form").requestSubmit();
    }
  }, []);
  return (
    <MockApi services={() => ({ network: new CreateServerService(network) })}>
      <ProfileSession signedIn={signedIn}>
        <div ref={ref}>
          {done ? (
            <p className="storybook__message">{done}</p>
          ) : (
            <AddNetworkModal
              onCreate={(res) => setDone(`created network ${res.network.id}`)}
              onClose={() => setDone("closed")}
            />
          )}
        </div>
      </ProfileSession>
    </MockApi>
  );
};

const stories: Story[] = [
  {
    name: "Default",
    component: () => <ModalStory />,
  },
  {
    name: "Signed out",
    component: () => <ModalStory signedIn={false} />,
  },
  {
    name: "Name required",
    component: () => <ModalStory signedIn={false} submit />,
  },
  {
    name: "Creating",
    component: () => <ModalStory network={{ delayMs: 4000 }} submit />,
  },
  {
    name: "Error",
    component: () => (
      <ModalStory network={{ refuse: "a network named test_user's Network exists" }} submit />
    ),
  },
];

export default stories;
