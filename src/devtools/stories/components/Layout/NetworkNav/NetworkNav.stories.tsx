// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../../../components/Layout/Layout.scss";

import React, { useEffect, useRef } from "react";
import { Route } from "react-router";

import NetworkNav from "../../../../../components/Layout/NetworkNav";
import { Provider as NetworkProvider } from "../../../../../contexts/Network";
import DirectoryService from "../../../../mocks/directory/service";
import NetworkService from "../../../../mocks/network/service";
import { MockApi, SessionProvider, StaticNetwork, Story, StoryRouter } from "../../../../storybook";
import { LayoutState, NetworkItem, networkItems, useNetworkIcons } from "../shell";

// Points at the `index`th network's gem, as a pointer over it would, so its
// tooltip shows.
const useHover = (root: React.RefObject<HTMLElement>, index?: number) =>
  useEffect(() => {
    if (index === undefined) {
      return;
    }
    let frame = 0;
    const hover = () => {
      const el = root.current?.querySelectorAll(".network_nav__item .network_nav__tooltip")[index];
      if (el) {
        el.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, relatedTarget: null }));
      } else {
        frame = requestAnimationFrame(hover);
      }
    };
    frame = requestAnimationFrame(hover);
    return () => cancelAnimationFrame(frame);
  }, []);

interface NavProps {
  expand?: boolean;
  hover?: number;
}

// The nav in the layout's nav column. Its links and the add network modal
// stay in the story.
const Nav: React.FC<NavProps> = ({ expand, hover }) => {
  const ref = useRef<HTMLDivElement>(null);
  useHover(ref, hover);
  return (
    <SessionProvider>
      <LayoutState expandNav={expand} />
      <StoryRouter>
        <Route
          path="/"
          element={
            <div className="layout_story__nav" ref={ref}>
              <div className="layout__nav">
                <NetworkNav />
              </div>
            </div>
          }
        />
      </StoryRouter>
    </SessionProvider>
  );
};

interface StaticNavProps extends NavProps {
  items: NetworkItem[];
  icons?: boolean;
}

// NetworkContext from `items`, with the fixtures' icons when `icons`.
const StaticNav: React.FC<StaticNavProps> = ({ items, icons, ...props }) => {
  const assetBundles = useNetworkIcons(icons ? items.length : 0);
  if (icons && !assetBundles) {
    return null;
  }
  return (
    <MockApi services={() => ({ network: new NetworkService({ count: items.length }) })}>
      <StaticNetwork items={items} assetBundles={assetBundles ?? undefined}>
        <Nav {...props} />
      </StaticNetwork>
    </MockApi>
  );
};

// The real Network provider over the network and directory mocks: their
// networks, peer counts and icons.
const LiveNav: React.FC<NavProps> = (props) => (
  <MockApi
    services={() => ({
      network: new NetworkService({ count: 8 }),
      directory: new DirectoryService(),
    })}
  >
    <NetworkProvider>
      <Nav {...props} />
    </NetworkProvider>
  </MockApi>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <LiveNav />,
  },
  {
    name: "Expanded",
    component: () => <LiveNav expand />,
  },
  {
    name: "Few",
    component: () => <StaticNav items={networkItems(3)} />,
  },
  {
    name: "Many",
    component: () => <StaticNav items={networkItems(30)} icons />,
  },
  {
    name: "Long names",
    component: () => <StaticNav items={networkItems(5, { long: true })} expand />,
  },
  {
    name: "No peers",
    component: () => <StaticNav items={networkItems(4, { peers: [0, 1, 0, 12] })} expand />,
  },
  {
    name: "Empty",
    component: () => <StaticNav items={[]} />,
  },
  {
    name: "Tooltip",
    component: () => <StaticNav items={networkItems(5, { long: true })} hover={1} />,
  },
];

export default stories;
