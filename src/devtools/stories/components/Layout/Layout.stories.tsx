// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.stories.scss";

import { Base64 } from "js-base64";
import React, { useContext, useEffect } from "react";
import { Route } from "react-router";

import DirectoryGrid from "../../../../components/Directory/Grid";
import Layout from "../../../../components/Layout";
import LayoutBody from "../../../../components/Layout/Body";
import { useLayout } from "../../../../contexts/Layout";
import { PlayerContext, PlayerMode, PlayerSource } from "../../../../contexts/Player";
import { Story, StoryRouter, fixtures } from "../../../storybook";
import { FollowLayout, LayoutState, ShellProviders } from "./shell";

const EMBED: PlayerSource = {
  type: "embed",
  service: "youtube",
  id: "oEgZeTr3Vs4",
  networkKey: "cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE",
};

const SWARM: PlayerSource = {
  type: "swarm",
  networkKey: Base64.fromUint8Array(fixtures.testNetworkKey, true),
  swarmUri: "magnet:?xt=urn:ppspp:Ef8K5Wl5wl6nkCLc7KiUOgcmT0UdJIttU4NVg7iRTgw",
  mimeType: "video/x.strims-units",
};

// How long the drawer stays open on the player before CloseVideo closes it.
const CLOSE_VIDEO_MS = 500;

// Closes the player as its close button does (Player's handleClose) once
// the drawer has opened and the chat beside the player has laid out.
const CloseVideo: React.FC = () => {
  const { overlayState, toggleShowVideo } = useLayout();
  const { setMode, setSource } = useContext(PlayerContext);
  const opened = overlayState.open && !overlayState.transitioning;
  useEffect(() => {
    if (opened) {
      const tid = setTimeout(() => {
        setMode(PlayerMode.CLOSED);
        setSource(null);
        toggleShowVideo(false);
      }, CLOSE_VIDEO_MS);
      return () => clearTimeout(tid);
    }
  }, [opened]);
  return null;
};

// Opens the composer's emote menu as its button does, once the chat has
// its composer.
const OpenEmoteMenu: React.FC = () => {
  useEffect(() => {
    let frame = 0;
    const open = () => {
      const button = document.querySelector<HTMLElement>(".layout__chat .chat_composer__button");
      if (button) {
        button.click();
      } else {
        frame = requestAnimationFrame(open);
      }
    };
    frame = requestAnimationFrame(open);
    return () => cancelAnimationFrame(frame);
  }, []);
  return null;
};

interface ShellStoryProps {
  room?: boolean;
  showChat?: boolean;
  source?: PlayerSource;
  mode?: PlayerMode;
  expandNav?: boolean;
  /**
   * The drawer open: on phones the panel of the player and chat pulled over
   * the content, else the player page's overlay, as opening a listing does.
   */
  open?: boolean;
  /** The player and chat on each other's side (alt+y). */
  swap?: boolean;
  theater?: boolean;
  fullScreenChat?: boolean;
  /** The player closed once the drawer has opened on it (CloseVideo). */
  closeVideo?: boolean;
  /** The composer's emote menu open (OpenEmoteMenu). */
  emoteMenu?: boolean;
}

// The whole shell as the app mounts it (MainProvider's providers, the
// layout around MainRouter's body route) at /, the directory grid in the
// body's outlet. Links navigate in the story. The flags are set in the
// layout's own context, and the frame around it, whose root carries its
// context's layout--* classes (which the layout's rules match too),
// follows them, through the story's steps and a swipe or a tap alike.
const ShellStory: React.FC<ShellStoryProps> = ({
  room,
  showChat,
  source,
  mode,
  expandNav,
  open,
  swap,
  theater,
  fullScreenChat,
  closeVideo,
  emoteMenu,
}) => {
  const frame = useLayout();
  return (
    <ShellProviders room={room} source={source} mode={mode}>
      <StoryRouter>
        <Route
          path="/"
          element={
            <Layout>
              <LayoutState
                showChat={showChat}
                showVideo={!!source}
                expandNav={expandNav}
                overlayOpen={open}
                swapMainPanels={swap}
                theaterMode={theater}
                fullScreenChat={fullScreenChat}
              />
              <FollowLayout frame={frame} />
              {closeVideo && <CloseVideo />}
              {emoteMenu && <OpenEmoteMenu />}
              <LayoutBody />
            </Layout>
          }
        >
          <Route index element={<DirectoryGrid listings={fixtures.directoryListings()} />} />
        </Route>
      </StoryRouter>
    </ShellProviders>
  );
};

// On phones and Android tablets the player and chat are a drawer over the
// content: closed, a sliver of it on the right; open (a swipe, or the
// chat's arrow), the player at its size and the chat beside or below it, or
// the chat alone. "Swapped" puts the chat on the player's other side, and
// "landscape cw" turns a landscape frame the other way round (the frame's
// own is ccw).
const stories: Story[] = [
  {
    name: "Home",
    component: () => <ShellStory />,
  },
  {
    name: "Home, chat open",
    component: () => <ShellStory open />,
  },
  {
    name: "Chat room",
    component: () => <ShellStory room />,
  },
  {
    name: "Chat room, swapped",
    component: () => <ShellStory room swap />,
  },
  {
    name: "Chat room, landscape cw",
    landscape: "cw",
    component: () => <ShellStory room />,
  },
  {
    name: "Chat open",
    component: () => <ShellStory room open />,
  },
  {
    name: "Chat open, swapped",
    component: () => <ShellStory room open swap />,
  },
  {
    name: "Chat open, landscape cw",
    landscape: "cw",
    component: () => <ShellStory room open />,
  },
  {
    name: "Chat open, swapped, landscape cw",
    landscape: "cw",
    component: () => <ShellStory room open swap />,
  },
  {
    name: "Chat open, emote menu",
    component: () => <ShellStory room open emoteMenu />,
  },
  {
    name: "Chat closed",
    component: () => <ShellStory room showChat={false} expandNav />,
  },
  {
    name: "Expanded nav",
    component: () => <ShellStory room expandNav />,
  },
  {
    name: "Full screen chat",
    component: () => <ShellStory room fullScreenChat />,
  },
  // The mini player: something plays while the directory shows.
  {
    name: "Playing",
    component: () => <ShellStory room source={EMBED} />,
  },
  {
    name: "Playing, swapped",
    component: () => <ShellStory room source={EMBED} swap />,
  },
  {
    name: "Playing, chat closed",
    component: () => <ShellStory room source={EMBED} showChat={false} />,
  },
  {
    name: "Playing large",
    component: () => <ShellStory room source={EMBED} mode={PlayerMode.LARGE} open />,
  },
  {
    name: "Playing large, swarm",
    component: () => <ShellStory room source={SWARM} mode={PlayerMode.LARGE} open />,
  },
  {
    name: "Playing large, swapped",
    component: () => <ShellStory room source={EMBED} mode={PlayerMode.LARGE} open swap />,
  },
  {
    name: "Playing large, landscape cw",
    landscape: "cw",
    component: () => <ShellStory room source={EMBED} mode={PlayerMode.LARGE} open />,
  },
  {
    name: "Playing large, emote menu",
    component: () => <ShellStory room source={EMBED} mode={PlayerMode.LARGE} open emoteMenu />,
  },
  {
    name: "Playing large, chat closed",
    component: () => (
      <ShellStory room source={EMBED} mode={PlayerMode.LARGE} open showChat={false} />
    ),
  },
  {
    name: "Theater mode",
    component: () => <ShellStory room source={SWARM} mode={PlayerMode.LARGE} open theater />,
  },
  {
    name: "Video closed",
    component: () => <ShellStory room source={SWARM} mode={PlayerMode.LARGE} open closeVideo />,
  },
  {
    name: "Video closed, swapped",
    component: () => (
      <ShellStory room source={SWARM} mode={PlayerMode.LARGE} open swap closeVideo />
    ),
  },
];

export default stories;
