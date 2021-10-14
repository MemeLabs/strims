import clsx from "clsx";
import React, { lazy, useCallback, useEffect, useState } from "react";
import { Redirect, Route, Switch, useHistory } from "react-router";
import { Link, NavLink } from "react-router-dom";
import { useAsyncFn, useToggle } from "react-use";

import Nav from "../components/Nav";

const stories = require.context("../stories/", true, /\.stories\.tsx$/, "lazy");

const Noop: React.FC = () => null;

type StoryModule = {
  default: React.Component;
};

type Nav = {
  name: string;
  path: string[];
  nodes: Nav[];
  module?: string;
  component?: React.Component;
};

type Extend = (path: string[], mod: StoryModule) => void;

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
    (path: string[], mod: StoryModule) =>
      setRoot((prev) => {
        let changed = false;
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
        node.nodes = [];
        for (const story of mod.default) {
          let next = nodes.find((p) => p.component === story.component);

          if (!next) {
            next = {
              name: story.name,
              path: [...path, story.name],
              nodes: [],
              component: story.component,
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

interface StorybookNavProps {
  node: Nav;
  extend: Extend;
}

const StorybookNav: React.FC<StorybookNavProps> = ({ node, extend }) => {
  const path = `/storybook/${node.path.join("/")}`;

  const history = useHistory();
  const [expanded, toggleExpanded] = useToggle(history.location.pathname.startsWith(path));

  useEffect(() => {
    if (expanded && node.module) {
      stories(node.module).then((mod) => extend(node.path, mod));
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
      <button onClick={() => toggleExpanded()}>{node.name}</button>
      <div
        className={clsx({
          "storybook_nav__content": true,
          "storybook_nav__content--expanded": expanded,
        })}
      >
        {node.nodes.map((node) => (
          <StorybookNav key={node.name} node={node} extend={extend} />
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

  const component = lazy(async () => {
    if (node.module) {
      const mod = await stories(node.module);
      extend(node.path, mod);
      const story = mod.default.find(
        (p) => `/storybook/${node.path.join("/")}/${p.name}` === history.location.pathname
      );
      if (story) {
        return { default: story.component };
      }
      const route = mod.default[0];
      return {
        default: () => <Redirect to={`/storybook/${node.path.join("/")}/${route.name}`} />,
      };
    }
    if (node.component) {
      return { default: node.component };
    }
    return null;
  });

  return (
    <>
      {node.nodes.map((node) => (
        <StorybookRoutes key={node.name} node={node} extend={extend} />
      ))}
      {node.module && <Route path={`/storybook/${node.path.join("/")}`} component={component} />}
    </>
  );
};

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
          <Switch>
            <StorybookRoutes node={node} extend={extend} />
          </Switch>
        </div>
      </div>
    </>
  );
};

export default Storybook;
