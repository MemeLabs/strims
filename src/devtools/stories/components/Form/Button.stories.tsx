// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Form.stories.scss";

import React from "react";
import { Route } from "react-router";

import { Button, ButtonSet } from "../../../../components/Form";
import InternalLink from "../../../../components/InternalLink";
import { Story, StoryRouter } from "../../../storybook";

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <div className="form_story form_story__row">
        <Button>Cancel</Button>
      </div>
    ),
  },
  {
    name: "Primary",
    component: () => (
      <div className="form_story form_story__row">
        <Button primary>Save</Button>
      </div>
    ),
  },
  {
    name: "Borderless",
    component: () => (
      <div className="form_story form_story__row">
        <Button borderless>Skip</Button>
      </div>
    ),
  },
  {
    name: "Disabled",
    component: () => (
      <div className="form_story form_story__row">
        <Button disabled>Cancel</Button>
        <Button primary disabled>
          Save
        </Button>
        <Button borderless disabled>
          Skip
        </Button>
      </div>
    ),
  },
  {
    name: "Long label",
    component: () => (
      <div className="form_story form_story__row">
        <Button primary>Create a network and invite everyone in this chat to it</Button>
      </div>
    ),
  },
  {
    name: "Button set",
    component: () => (
      <StoryRouter>
        <Route
          path="/"
          element={
            <>
              <form className="form_story thing_form" onSubmit={(e) => e.preventDefault()}>
                <ButtonSet>
                  <InternalLink to="/settings">Back to settings</InternalLink>
                  <Button>Cancel</Button>
                  <Button primary>Save</Button>
                </ButtonSet>
              </form>
              <div className="form_story">
                <ButtonSet>
                  <Button borderless>Skip</Button>
                  <Button primary>Continue</Button>
                </ButtonSet>
              </div>
            </>
          }
        />
      </StoryRouter>
    ),
  },
];

export default stories;
