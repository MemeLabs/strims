// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The app's dropdowns take their look from Settings/Table.scss's base classes.
import "../../../../components/Settings/Table.scss";
import "../core.stories.scss";

import React, { ReactNode, useEffect, useRef } from "react";
import { BsThreeDots } from "react-icons/bs";

import Dropdown from "../../../../components/Dropdown";
import { Story } from "../../../storybook";

const items = (labels: string[], className: string) =>
  labels.map((label) => (
    <button key={label} className={className}>
      {label}
    </button>
  ));

// Opens the dropdown inside it once mounted, as a click on its anchor would.
// The app's menus open leftward from the right of a title bar or table row:
// the row ends where the story's column does.
const Opened: React.FC<{ children: ReactNode }> = ({ children }) => {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => ref.current.querySelector<HTMLElement>(".dropdown__anchor").click(), []);
  return (
    <div ref={ref} className="core_story__row core_story__row--end">
      {children}
    </div>
  );
};

const stories: Story[] = [
  {
    name: "Closed",
    component: () => (
      <div className="core_story">
        <div className="core_story__row core_story__row--end">
          <Dropdown
            baseClassName="thing_table_dropdown"
            anchor="Add network"
            items={items(["Create new network", "Add invite code"], "thing_table_dropdown__button")}
          />
        </div>
      </div>
    ),
  },
  {
    name: "Open",
    component: () => (
      <div className="core_story">
        <Opened>
          <Dropdown
            baseClassName="thing_table_dropdown"
            anchor="Add network"
            items={items(["Create new network", "Add invite code"], "thing_table_dropdown__button")}
          />
        </Opened>
      </div>
    ),
  },
  {
    name: "Item menu",
    component: () => (
      <div className="core_story">
        <Opened>
          <Dropdown
            baseClassName="thing_table_item_dropdown"
            anchor={<BsThreeDots />}
            items={items(
              ["Create invite", "Publish", "Delete"],
              "thing_table_item_dropdown__button"
            )}
          />
        </Opened>
      </div>
    ),
  },
  {
    name: "Many items",
    component: () => (
      <div className="core_story">
        <Opened>
          <Dropdown
            baseClassName="thing_table_dropdown"
            anchor="Move to network"
            items={items(
              Array.from({ length: 12 }, (_, i) => `Network ${i + 1}`),
              "thing_table_dropdown__button"
            )}
          />
        </Opened>
      </div>
    ),
  },
  {
    name: "Unstyled",
    component: () => (
      <div className="core_story">
        <p className="core_story__caption">
          Without a base class the menu has Dropdown.scss&apos;s placement alone.
        </p>
        <Opened>
          <Dropdown anchor="Options" items={items(["First", "Second"], "")} />
        </Opened>
      </div>
    ),
  },
];

export default stories;
