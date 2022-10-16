// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { ReactNode, Suspense, lazy, useCallback, useEffect, useState } from "react";
import { MdFullscreen } from "react-icons/md";
import { Location, Navigate, Route, Routes, useLocation } from "react-router";
import { NavLink } from "react-router-dom";
import { useSessionStorage, useToggle } from "react-use";

import { WithThemeProps, withTheme } from "../../components/Theme";
import { WithRootRefProps, withLayoutContext } from "../../contexts/Layout";
import { Provider as ThemeProvider } from "../../contexts/Theme";
import Nav from "../components/Nav";

const stories = require.context("../stories/", true, /\.stories\.tsx$/, "lazy");

type StoriesModule = {
  default: {
    name: string;
    component: React.ComponentType;
  }[];
};

type Nav = {
  name: string;
  path: string[];
  nodes: Nav[];
  module?: string;
  component?: React.ComponentType;
};

type Extend = (path: string[], mod: StoriesModule) => void;

const useStorybookNav = (): [Nav, Extend] => {
  const [root, setRoot] = useState(() => {
    const root: Nav = {
      name: "root",
      path: [],
      nodes: [],
    };
    for (const module of stories.keys().sort((a, b) => a.localeCompare(b))) {
      let node = root;
      let path: string[] = [];
      const parts = module.replace(/(\.\/|\.stories.tsx)/g, "").split("/");

      for (const name of parts) {
        path = [...path, name];
        let next = node.nodes.find((p) => p.name === name);
        if (!next) {
          next = {
            name,
            path,
            nodes: [],
          };
          node.nodes.push(next);
        }
        node = next;
      }
      node.module = module;
    }
    return root;
  });

  const extend = useCallback(
    (path: string[], mod: StoriesModule) =>
      setRoot((prev) => {
        const root = { ...prev };
        let node = root;
        for (const name of path) {
          const i = node.nodes.findIndex((p) => p.name === name);
          node.nodes = [...node.nodes];
          const next = { ...node.nodes[i] };
          node.nodes[i] = next;
          node = next;
        }

        const nodes = node.nodes;
        let changed = nodes.length !== mod.default.length;
        node.nodes = [];
        for (const story of mod.default) {
          let next = nodes.find((p) => p.component === story.component);

          if (!next) {
            next = {
              ...story,
              path: [...path, story.name],
              nodes: [],
            };
            changed = true;
          }

          node.nodes.push(next);
        }
        return changed ? root : prev;
      }),
    []
  );

  return [root, extend];
};

const formatPath = (...parts: string[]) => "/storybook/" + parts.join("/");

interface StorybookNavProps {
  node: Nav;
  shouldExpand?: boolean;
  extend: Extend;
}

const StorybookNav: React.FC<StorybookNavProps> = ({ node, shouldExpand = true, extend }) => {
  const path = formatPath(...node.path);

  const location = useLocation();
  const [expanded, toggleExpanded] = useToggle(shouldExpand || location.pathname.startsWith(path));

  useEffect(() => {
    if (expanded && node.module) {
      const mod = stories(node.module) as Promise<StoriesModule>;
      void mod.then((mod) => extend(node.path, mod));
    }
  }, [expanded, node, stories]);

  if (node.component) {
    return (
      <NavLink
        className={({ isActive }) =>
          clsx({
            "storybook_nav__link": true,
            "storybook_nav__link--active": isActive,
          })
        }
        to={path}
      >
        {node.name}
      </NavLink>
    );
  }

  return (
    <div className={"storybook_nav"}>
      <NavLink to={path} onClick={() => toggleExpanded()}>
        {node.name}
      </NavLink>
      <div
        className={clsx({
          "storybook_nav__content": true,
          "storybook_nav__content--expanded": expanded,
        })}
      >
        {node.nodes.map((node) => (
          <StorybookNav key={node.name} node={node} shouldExpand={false} extend={extend} />
        ))}
      </div>
    </div>
  );
};

const storybookRoutes = (location: Location, node: Nav, extend: Extend): React.ReactElement[] => {
  const C = lazy(async (): Promise<{ default: React.ComponentType }> => {
    const mod = (await stories(node.module)) as StoriesModule;
    extend(node.path, mod);
    const story = mod.default.find((p) => formatPath(...node.path, p.name) === location.pathname);
    if (story) {
      return { default: story.component };
    }
    const route = mod.default[0];
    return {
      default: () => <Navigate to={formatPath(...node.path, route.name)} />,
    };
  });

  const routes = node.nodes.map((node) => storybookRoutes(location, node, extend)).flat();
  if (node.module) {
    routes.push(<Route key={node.name} path={node.path.join("/") + "/*"} element={<C />} />);
  }
  return routes;
};

interface StoryContainerProps extends WithThemeProps, WithRootRefProps {
  children: ReactNode;
}

const StoryContainerBase: React.FC<StoryContainerProps> = ({ className, rootRef, children }) => (
  <div className={clsx(className, "storybook__content layout--pc")} ref={rootRef}>
    {children}
  </div>
);

const StoryContainer = withTheme(withLayoutContext(StoryContainerBase));

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Storybook: React.FC = () => {
  const [node, extend] = useStorybookNav();
  const location = useLocation();
  const [fullscreen, setFullscreen] = useSessionStorage("storybook_fullscreen", false);
  const toggleFullscreen = useCallback(() => setFullscreen(!fullscreen), [fullscreen]);

  return (
    <>
      <Nav />
      <div
        className={clsx({
          "storybook": true,
          "storybook--fullscreen": fullscreen,
        })}
      >
        <div className="storybook__nav">
          <StorybookNav node={node} extend={extend} />
        </div>
        <ThemeProvider>
          <StoryContainer>
            <div className="storybook__toolbar">
              <button className="storybook__toolbar_button" onClick={toggleFullscreen}>
                <MdFullscreen />
              </button>
            </div>
            <Suspense fallback={<LoadingMessage />}>
              <Routes>{storybookRoutes(location, node, extend)}</Routes>
            </Suspense>
          </StoryContainer>
        </ThemeProvider>
      </div>
    </>
  );
};

export default Storybook;
