// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Form.stories.scss";

import React from "react";

import { InputError } from "../../../../components/Form";
import { Story } from "../../../storybook";

const stories: Story[] = [
  {
    name: "String",
    component: () => (
      <div className="form_story">
        <InputError error="Name is required" />
      </div>
    ),
  },
  {
    name: "Error",
    component: () => (
      <div className="form_story">
        <InputError error={new Error("rpc error: network not found")} />
      </div>
    ),
  },
  {
    name: "Field error",
    component: () => (
      <div className="form_story">
        <InputError error={{ type: "pattern", message: "Name contains invalid characters" }} />
      </div>
    ),
  },
  {
    name: "No message",
    component: () => (
      <div className="form_story">
        <InputError error={{ type: "validate" }} />
      </div>
    ),
  },
  {
    name: "Long text",
    component: () => (
      <div className="form_story">
        <InputError
          error={
            "rpc error: the certificate authority refused the renewal because the alias test_user " +
            "is held by another key in this network; join again under another name or ask the " +
            "network's owner to release the reservation"
          }
        />
      </div>
    ),
  },
];

export default stories;
