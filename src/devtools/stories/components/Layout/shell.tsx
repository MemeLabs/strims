// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.stories.scss";

import { PassThrough } from "stream";

import React, { ReactNode, useEffect, useMemo, useState } from "react";

import { LinkedProfile } from "../../../../apis/strims/auth/v1/auth";
import {
  AssetBundle,
  FrontendWatchListingsRequest,
  FrontendWatchListingsResponse,
  IListingSnippet,
  INetworkListingsItem,
  Listing,
} from "../../../../apis/strims/network/v1/directory/directory";
import { Network } from "../../../../apis/strims/network/v1/network";
import { ImageType } from "../../../../apis/strims/type/image";
import { Provider as ChatProvider, useChat } from "../../../../contexts/Chat";
import { LayoutContextProps, useLayout } from "../../../../contexts/Layout";
import { Provider as NetworkProvider } from "../../../../contexts/Network";
import { PlayerMode, PlayerSource } from "../../../../contexts/Player";
import { Ops, SessionContext, State } from "../../../../contexts/Session";
import Emitter from "../../../mocks/chat/MessageEmitter";
import ChatService from "../../../mocks/chat/service";
import DirectoryService from "../../../mocks/directory/service";
import NetworkService from "../../../mocks/network/service";
import NotificationService from "../../../mocks/notification/service";
import { MockApi, MockNotifications, StaticPlayer, fixtures } from "../../../storybook";

// The chat mock's room's key (listing 2's chat server, in network 1).
const roomKey = (): Uint8Array => {
  const { content } = new Listing(fixtures.listingItem(BigInt(2)).listing);
  return content.case === Listing.ContentCase.CHAT ? content.chat.key : null;
};

// Opens the chat mock's room in the main chat, as picking it in the room
// menu would, and renders `children` once it's the main chat's (the chat
// shows its room menu, open, while it has none).
const OpenRoom: React.FC<{ children: ReactNode }> = ({ children }) => {
  const [{ mainActiveTopic }, { openRoom, setMainActiveTopic }] = useChat();
  useEffect(() => {
    const key = roomKey();
    openRoom(key, fixtures.testNetworkKey);
    setMainActiveTopic({ type: "ROOM", topicKey: key });
  }, []);
  return mainActiveTopic ? <>{children}</> : null;
};

export interface ShellProvidersProps {
  /** Opens the chat's room (else its room menu shows). */
  room?: boolean;
  /** What the player plays, with the meta of the fixtures' listing 36. */
  source?: PlayerSource;
  mode?: PlayerMode;
  children: ReactNode;
}

// What the app's shell (MainProvider) gives the layout, over the network,
// directory, chat and notification mocks: 8 networks, the fixtures'
// listings, the chat mock's room with 40 of its messages, no notifications.
export const ShellProviders: React.FC<ShellProvidersProps> = ({
  room,
  source,
  mode = PlayerMode.PIP,
  children,
}) => (
  <MockApi
    services={() => ({
      network: new NetworkService({ count: 8 }),
      directory: new ListingDirectoryService(),
      chat: new ChatService(new Emitter({ ivl: 100, limit: 40, preload: 40 })),
      notification: new NotificationService(),
    })}
  >
    <ProfileSession>
      <NetworkProvider>
        <MockNotifications>
          <ChatProvider>
            <StaticPlayer
              value={
                source
                  ? {
                      source,
                      mode,
                      path: "/embed/youtube/oEgZeTr3Vs4",
                      networkKey: fixtures.testNetworkKey,
                      listingId: BigInt(36),
                    }
                  : undefined
              }
            >
              {room ? <OpenRoom>{children}</OpenRoom> : children}
            </StaticPlayer>
          </ChatProvider>
        </MockNotifications>
      </NetworkProvider>
    </ProfileSession>
  </MockApi>
);

// The directory mock, but watchListings for one listing (as the player's
// meta asks) answers that listing alone, `snippet` over its own; other
// watches answer all of the fixtures' listings.
export class ListingDirectoryService extends DirectoryService {
  constructor(
    private snippet: IListingSnippet = {},
    private userCount = 1234
  ) {
    super();
  }

  watchListings(req: FrontendWatchListingsRequest) {
    if (!req.listingId) {
      return super.watchListings(req);
    }
    const ch = new PassThrough({ objectMode: true });
    const item = fixtures.listingItem(req.listingId);
    if (item) {
      const listing: INetworkListingsItem = {
        ...item,
        snippet: { ...item.snippet, ...this.snippet },
        moderation: {},
        userCount: this.userCount,
        recentUserCount: 0,
      };
      ch.push(
        new FrontendWatchListingsResponse({
          events: [
            {
              event: {
                change: { listings: { network: fixtures.listingsNetwork(), listings: [listing] } },
              },
            },
          ],
        })
      );
    }
    return ch;
  }
}

export interface ProfileSessionProps {
  /** Signed in as the fixtures' profile (default), or signed out. */
  signedIn?: boolean;
  linkedProfiles?: LinkedProfile[];
  children: ReactNode;
}

// A session signed in as the fixtures' profile (test_user), or signed out
// with `linkedProfiles` stored; sign-ins succeed and sign-ups store nothing.
export const ProfileSession: React.FC<ProfileSessionProps> = ({
  signedIn = true,
  linkedProfiles = [],
  children,
}) => {
  const value = useMemo<[State, Ops]>(
    () => [
      { linkedProfiles, profile: signedIn ? fixtures.profile() : null, loading: false },
      {
        createProfile: () => Promise.resolve(null),
        signIn: () => Promise.resolve(true),
        loadLinkedProfiles: () => Promise.resolve(linkedProfiles),
      },
    ],
    []
  );
  return <SessionContext.Provider value={value}>{children}</SessionContext.Provider>;
};

type LayoutFlags = Partial<
  Pick<
    LayoutContextProps,
    "showChat" | "showVideo" | "theaterMode" | "expandNav" | "swapMainPanels"
  >
> & {
  /** The content panel's overlay (the player over the content) open. */
  overlayOpen?: boolean;
};

// Sets the nearest layout context's flags on mount, as its toggles would,
// and puts them back after.
export const LayoutState: React.FC<LayoutFlags> = ({
  showChat,
  showVideo,
  theaterMode,
  expandNav,
  swapMainPanels,
  overlayOpen,
}) => {
  const layout = useLayout();
  useEffect(() => {
    const toggles: [boolean | undefined, (v: boolean) => void][] = [
      [showChat, layout.toggleShowChat],
      [showVideo, layout.toggleShowVideo],
      [theaterMode, layout.toggleTheaterMode],
      [expandNav, layout.toggleExpandNav],
      [swapMainPanels, layout.toggleSwapMainPanels],
      [overlayOpen, layout.toggleOverlayOpen],
    ];
    const set = toggles.filter(([v]) => v !== undefined);
    set.forEach(([v, toggle]) => toggle(v));
    return () => {
      layout.toggleShowChat(true);
      layout.toggleShowVideo(false);
      layout.toggleTheaterMode(false);
      layout.toggleExpandNav(false);
      layout.toggleSwapMainPanels(false);
      layout.toggleOverlayOpen(false);
    };
  }, []);
  return null;
};

export type NetworkItem = { network: Network; peerCount: number };

const LONG_NAMES = [
  "The Extremely Long Network Name That Keeps Going",
  "Speedrunners Anonymous: Any% Glitchless Division",
  "Comfy",
  "Сеть с длинным названием на кириллице",
  "日本語のとても長いネットワークの名前です",
];

// Networks 1..count with the fixtures' names, or long ones; peer counts
// from `peers` (default 2^i).
export const networkItems = (
  count: number,
  { long = false, peers }: { long?: boolean; peers?: number[] } = {}
): NetworkItem[] =>
  Array.from({ length: count }, (_, i) => ({
    network: fixtures.network(i + 1, long ? { name: LONG_NAMES[i % LONG_NAMES.length] } : {}),
    peerCount: peers?.[i % peers.length] ?? Math.pow(2, i),
  }));

// Icons for networks 1..count from the fixtures' square images, loaded
// once: null until they are.
export const useNetworkIcons = (count: number): Map<bigint, AssetBundle> | null => {
  const [bundles, setBundles] = useState<Map<bigint, AssetBundle>>(null);
  useEffect(() => {
    let live = true;
    void Promise.all(
      fixtures.iconUrls.slice(0, count).map(async (url, i) => {
        const data = await fixtures.imageData(url);
        return [
          BigInt(i + 1),
          new AssetBundle({ icon: { type: ImageType.IMAGE_TYPE_PNG, data } }),
        ] as const;
      })
    ).then((entries) => live && setBundles(new Map(entries)));
    return () => {
      live = false;
    };
  }, []);
  return bundles;
};
