// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoPlayer/VideoPlayer.stories.scss";

import React, { useEffect, useRef, useState } from "react";

import EmbedPlayer, { EmbedPlayerProps } from "../../../components/EmbedPlayer";
import { Story } from "../../storybook";

// The service's own player in an iframe, as the layout's player column
// holds it, and the URL it loads (from the service, when the browser can
// reach it).
const Embed: React.FC<EmbedPlayerProps> = (props) => {
  const ref = useRef<HTMLDivElement>(null);
  const [src, setSrc] = useState("");
  useEffect(() => setSrc(ref.current.querySelector("iframe")?.src ?? ""), []);
  return (
    <div>
      <div className="video_story__swarm" ref={ref}>
        <EmbedPlayer {...props} />
      </div>
      <p className="video_story__caption">
        <code>{src}</code>
      </p>
    </div>
  );
};

const stories: Story[] = [
  {
    name: "AngelThump",
    component: () => <Embed service="angelthump" id="eastmancolor" />,
  },
  {
    name: "Twitch",
    component: () => <Embed service="twitch" id="destiny" />,
  },
  {
    name: "Twitch VOD",
    component: () => (
      <Embed service="twitch-vod" id="1234567890" queryParams={new Map([["t", "1h2m3s"]])} />
    ),
  },
  {
    name: "YouTube",
    component: () => (
      <Embed service="youtube" id="oEgZeTr3Vs4" queryParams={new Map([["t", "90"]])} />
    ),
  },
];

export default stories;
