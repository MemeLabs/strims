// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./core.stories.scss";

import React from "react";

import ExternalLink from "../../../components/ExternalLink";
import { Story } from "../../storybook";

const stories: Story[] = [
  {
    name: "Valid",
    component: () => (
      <div className="core_story">
        <ExternalLink href="https://strims.gg/directory">https://strims.gg/directory</ExternalLink>
      </div>
    ),
  },
  {
    // Not a URL: a span, not a link.
    name: "Invalid",
    component: () => (
      <div className="core_story">
        <ExternalLink href="javascript:alert(1)">javascript:alert(1)</ExternalLink>
        <ExternalLink href="not a url">not a url</ExternalLink>
      </div>
    ),
  },
  {
    name: "In text",
    component: () => (
      <div className="core_story">
        <p className="core_story__text">
          The schedule is at{" "}
          <ExternalLink href="https://example.com/schedule?week=12">
            example.com/schedule
          </ExternalLink>{" "}
          and the archive at{" "}
          <ExternalLink href="ftp:/example.com/archive">ftp:/example.com/archive</ExternalLink>,
          which isn&apos;t a valid address, so it stays plain text.
        </p>
      </div>
    ),
  },
  {
    name: "Long URL",
    component: () => (
      <div className="core_story">
        <p className="core_story__text">
          <ExternalLink href="https://example.com/a/very/long/path/that/keeps/going/past/the/width/of/its/column?with=query&and=more&parameters=too">
            https://example.com/a/very/long/path/that/keeps/going/past/the/width/of/its/column?with=query&and=more&parameters=too
          </ExternalLink>
        </p>
      </div>
    ),
  },
];

export default stories;
