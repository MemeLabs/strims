// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Form.stories.scss";

import React, { ReactNode } from "react";

import { ImageInputPlaceholder, InputLabel } from "../../../../components/Form";
import { Story } from "../../../storybook";

const Form: React.FC<{ bare?: boolean; children: ReactNode }> = ({ bare, children }) => (
  <form
    className={bare ? "form_story" : "form_story thing_form"}
    onSubmit={(e) => e.preventDefault()}
  >
    {children}
  </form>
);

const Input: React.FC<{ value?: string }> = ({ value = "test_user" }) => (
  <input className="input input_text" defaultValue={value} />
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => (
      <Form>
        <InputLabel text="Profile name">
          <Input />
        </InputLabel>
      </Form>
    ),
  },
  {
    name: "Required",
    component: () => (
      <Form>
        <InputLabel text="Profile name" required>
          <Input />
        </InputLabel>
      </Form>
    ),
  },
  {
    name: "Description",
    component: () => (
      <Form>
        <InputLabel
          text="Profile name"
          description="3 to 20 letters, digits or underscores."
          required
        >
          <Input />
        </InputLabel>
      </Form>
    ),
  },
  {
    name: "Inline input",
    component: () => (
      <Form>
        <InputLabel text="Pairing token" inlineInput>
          <button className="input_button input_button--primary">Paste</button>
        </InputLabel>
      </Form>
    ),
  },
  {
    name: "Div",
    component: () => (
      <Form>
        <InputLabel text="Image" component="div" description="Rendered as a div, not a label.">
          <div className="input_image">
            <ImageInputPlaceholder classNameBase="input_image" />
          </div>
        </InputLabel>
      </Form>
    ),
  },
  {
    name: "Input types",
    component: () => (
      <Form>
        {["default", "text", "textarea", "select", "toggle"].map((inputType) => (
          <InputLabel key={inputType} text={inputType} inputType={inputType}>
            <Input value={`input_label--${inputType}`} />
          </InputLabel>
        ))}
      </Form>
    ),
  },
  {
    name: "Bare",
    component: () => (
      <Form bare>
        <InputLabel text="Profile name" required>
          <Input />
        </InputLabel>
        <InputLabel text="Pairing token" inlineInput>
          <button className="input_button input_button--primary">Paste</button>
        </InputLabel>
      </Form>
    ),
  },
  {
    name: "Long text",
    component: () => (
      <Form>
        <InputLabel
          text="A label long enough that it wraps across more than one line of its column"
          description={
            "A description long enough to wrap: the server rejects names other members of the " +
            "network already hold, and names change only when the certificate renews."
          }
        >
          <Input />
        </InputLabel>
      </Form>
    ),
  },
];

export default stories;
