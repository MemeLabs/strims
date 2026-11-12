// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Settings.stories.scss";

import React from "react";

import Label from "../../../../components/Settings/Label";
import { Table, TableCell } from "../../../../components/Settings/Table";
import { Story } from "../../../storybook";

const emotes: [string, string[]][] = [
  ["ComfyCat", ["animated", "wide"]],
  ["BasedGod", []],
  ["FeelsStrongMan", ["seasonal", "a label long enough to crowd its cell"]],
];

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <div className="settings_story">
        <div>
          <Label>animated</Label>
        </div>
      </div>
    ),
  },
  {
    name: "Several",
    component: () => (
      <div className="settings_story">
        <div>
          {["animated", "wide", "seasonal", "sub only"].map((l) => (
            <Label key={l}>{l}</Label>
          ))}
        </div>
      </div>
    ),
  },
  {
    name: "In a table",
    component: () => (
      <div className="settings_story">
        <Table>
          <tbody>
            {emotes.map(([name, labels]) => (
              <tr key={name}>
                <TableCell>{name}</TableCell>
                <TableCell>
                  {labels.map((l) => (
                    <Label key={l}>{l}</Label>
                  ))}
                </TableCell>
              </tr>
            ))}
          </tbody>
        </Table>
      </div>
    ),
  },
];

export default stories;
