// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoPlayer.stories.scss";

import { PassThrough } from "stream";

import { Base64 } from "js-base64";
import React, { useEffect } from "react";

import { UnimplementedEgressService } from "../../../../apis/strims/video/v1/egress_rpc";
import {
  HLSEgressGetConfigResponse,
  HLSEgressIsSupportedResponse,
} from "../../../../apis/strims/video/v1/hls_egress";
import { UnimplementedHLSEgressService } from "../../../../apis/strims/video/v1/hls_egress_rpc";
import VideoPlayer from "../../../../components/VideoPlayer";
import { useLayout } from "../../../../contexts/Layout";
import {
  Catalog,
  MockApi,
  SessionProvider,
  StaticPlayer,
  Story,
  fixtures,
} from "../../../storybook";

// The node serves no HLS, so the player offers no Cast or AirPlay button.
class HLSEgressService extends UnimplementedHLSEgressService {
  isSupported() {
    return new HLSEgressIsSupportedResponse({ supported: false });
  }

  getConfig() {
    return new HLSEgressGetConfigResponse({ config: { enabled: false } });
  }
}

// A stream that opens and sends nothing: the swarm has no data yet.
class EgressService extends UnimplementedEgressService {
  private streams: PassThrough[] = [];

  openStream() {
    const ch = new PassThrough({ objectMode: true });
    this.streams.push(ch);
    return ch;
  }

  destroy() {
    this.streams.forEach((ch) => ch.destroy());
  }
}

const NETWORK_KEY = Base64.fromUint8Array(fixtures.testNetworkKey, true);
const SWARM_URI = "magnet:?xt=urn:ppspp:Ef8K5Wl5wl6nkCLc7KiUOgcmT0UdJIttU4NVg7iRTgw";
const UNITS = "video/x.strims-units";

const TheaterMode: React.FC = () => {
  const { toggleTheaterMode } = useLayout();
  useEffect(() => {
    toggleTheaterMode(true);
    return () => toggleTheaterMode(false);
  }, []);
  return null;
};

interface PlayerStoryProps {
  mimeType?: string;
  controls?: boolean;
  disableControls?: boolean;
  theaterMode?: boolean;
}

// The swarm player as the layout's player column holds it, its controls
// shown; nothing it is given plays here (no node, no player worker).
const PlayerStory: React.FC<PlayerStoryProps> = ({
  mimeType = UNITS,
  controls = true,
  disableControls,
  theaterMode,
}) => (
  <MockApi
    services={() => ({ hlsEgress: new HLSEgressService(), videoEgress: new EgressService() })}
  >
    <SessionProvider>
      <StaticPlayer>
        {theaterMode && <TheaterMode />}
        <div className="video_story__swarm">
          <VideoPlayer
            networkKey={NETWORK_KEY}
            swarmUri={SWARM_URI}
            mimeType={mimeType}
            defaultControlsVisible={controls}
            disableControls={disableControls}
            handleClose={() => undefined}
          />
        </div>
      </StaticPlayer>
    </SessionProvider>
  </MockApi>
);

const stories: Story[] = [
  {
    name: "Loading",
    component: () => <PlayerStory />,
  },
  {
    name: "Unplayable format",
    component: () => <PlayerStory mimeType="video/mp2t" />,
  },
  {
    name: "Controls hidden",
    component: () => <PlayerStory controls={false} />,
  },
  {
    name: "Picture in picture",
    component: () => <PlayerStory disableControls />,
  },
  {
    name: "Theater mode",
    component: () => <PlayerStory theaterMode />,
  },
];

export default stories;

export const catalog: Catalog = {
  skips: {
    "components/VideoPlayer/SwarmMedia.tsx":
      "UnitsMedia renders nothing: it feeds the video from a node's feeds or the Egress stream through the player worker, which the console build stubs (no wasm); the VideoPlayer stories mount it",
    "components/VideoPlayer/RemotePlayback.tsx":
      "a hook (useRemotePlayback), no component: its Cast and AirPlay buttons show only where the browser can cast or AirPlay to a receiver and the node serves HLS, which headless Chromium and the mocks don't have",
  },
};
