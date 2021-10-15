import clsx from "clsx";
import React, { Suspense, lazy, useCallback, useEffect, useState } from "react";
import { Redirect, Route, Switch, useHistory } from "react-router";
import { NavLink } from "react-router-dom";
import { useToggle } from "react-use";

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

  const history = useHistory();
  const [expanded, toggleExpanded] = useToggle(
    shouldExpand || history.location.pathname.startsWith(path)
  );

  useEffect(() => {
    if (expanded && node.module) {
      const mod = stories(node.module) as Promise<StoriesModule>;
      void mod.then((mod) => extend(node.path, mod));
    }
  }, [expanded, node, stories]);

  if (node.component) {
    return (
      <NavLink
        className="storybook_nav__link"
        activeClassName="storybook_nav__link--active"
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

interface StorybookRoutesProps {
  node: Nav;
  extend: Extend;
}

const StorybookRoutes: React.FC<StorybookRoutesProps> = ({ node, extend }) => {
  const history = useHistory();

  const component = lazy(async (): Promise<{ default: React.ComponentType }> => {
    const mod = (await stories(node.module)) as StoriesModule;
    extend(node.path, mod);
    const story = mod.default.find(
      (p) => formatPath(...node.path, p.name) === history.location.pathname
    );
    if (story) {
      return { default: story.component };
    }
    const route = mod.default[0];
    return {
      default: () => <Redirect to={formatPath(...node.path, route.name)} />,
    };
  });

  return (
    <>
      {node.nodes.map((node) => (
        <StorybookRoutes key={node.name} node={node} extend={extend} />
      ))}
      {node.module && <Route path={formatPath(...node.path)} component={component} />}
    </>
  );
};

const LoadingMessage = () => <p className="loading_message">loading</p>;

const Storybook: React.FC = () => {
  const [node, extend] = useStorybookNav();

  return (
    <>
      <Nav />
      <div className="storybook">
        <div className="storybook__nav">
          <StorybookNav node={node} extend={extend} />
        </div>
        <div className="storybook__content">
          <Suspense fallback={<LoadingMessage />}>
            <Switch>
              <StorybookRoutes node={node} extend={extend} />
            </Switch>
          </Suspense>
        </div>
      </div>
    </>
  );
};

export default Storybook;
