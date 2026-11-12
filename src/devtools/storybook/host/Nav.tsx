// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import clsx from "clsx";
import React, { useState } from "react";
import { NavLink, useLocation } from "react-router";

import { IndexData, ROOT, moduleKeys, modulePath, storyRoute } from "./catalog";

interface Node {
  name: string;
  path: string;
  children: Node[];
  key?: string;
}

const tree = ((): Node => {
  const root: Node = { name: "", path: "", children: [] };
  for (const key of moduleKeys) {
    let node = root;
    for (const name of modulePath(key).split("/")) {
      const path = node.path ? `${node.path}/${name}` : name;
      let next = node.children.find((c) => c.name === name);
      if (!next) {
        next = { name, path, children: [] };
        node.children.push(next);
      }
      node = next;
    }
    node.key = key;
  }
  return root;
})();

const linkClassName = ({ isActive }: { isActive: boolean }) =>
  clsx("storybook_nav__link", { "storybook_nav__link--active": isActive });

type IndexProp = { index: IndexData | null };

// The module's stories, from the index (null while it loads).
const ModuleStories: React.FC<{ moduleKey: string } & IndexProp> = ({ moduleKey, index }) => {
  const { search } = useLocation();
  const module = modulePath(moduleKey);
  if (!index) {
    return <span className="storybook_nav__loading">loading</span>;
  }
  const error = index.modules.find((m) => m.module === module)?.error;
  if (error) {
    return <span className="storybook_nav__error">{error}</span>;
  }
  return (
    <>
      {index.stories
        .filter((s) => s.module === module)
        .map(({ name }) => (
          <NavLink
            key={name}
            className={linkClassName}
            to={{ pathname: storyRoute(moduleKey, name), search }}
          >
            {name}
          </NavLink>
        ))}
    </>
  );
};

const NavNode: React.FC<{ node: Node } & IndexProp> = ({ node, index }) => {
  const { pathname } = useLocation();
  const route = `${ROOT}/${node.path}`;
  const [expanded, setExpanded] = useState(
    () => pathname === route || pathname.startsWith(route + "/")
  );

  return (
    <div className="storybook_nav">
      <button
        className={clsx("storybook_nav__toggle", { "storybook_nav__toggle--module": node.key })}
        onClick={() => setExpanded(!expanded)}
      >
        {expanded ? "▾" : "▸"} {node.name}
      </button>
      <div
        className={clsx({
          "storybook_nav__content": true,
          "storybook_nav__content--expanded": expanded,
        })}
      >
        {expanded && node.key && <ModuleStories moduleKey={node.key} index={index} />}
        {expanded &&
          node.children.map((child) => <NavNode key={child.name} node={child} index={index} />)}
      </div>
    </div>
  );
};

// The story modules' tree, below a link to the catalog.
const StorybookNav: React.FC<IndexProp> = ({ index }) => {
  const { search } = useLocation();
  return (
    <>
      <NavLink className={linkClassName} to={{ pathname: ROOT, search }} end>
        Catalog
      </NavLink>
      {tree.children.map((node) => (
        <NavNode key={node.name} node={node} index={index} />
      ))}
    </>
  );
};

export default StorybookNav;
