// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./VideoPlayer.stories.scss";

import React from "react";

import LogoButton, { LogoButtonProps } from "../../../../components/VideoPlayer/LogoButton";
import { Story } from "../../../storybook";

const Button: React.FC<LogoButtonProps> = (props) => (
  <div className="video_story__logo">
    <LogoButton {...props} onClick={() => null} />
  </div>
);

const stories: Story[] = [
  {
    name: "Default",
    component: () => <Button />,
  },
  {
    name: "Spin",
    component: () => <Button spin />,
  },
  {
    name: "Flicker",
    component: () => <Button flicker />,
  },
  {
    name: "Pulse",
    component: () => <Button pulse />,
  },
  {
    name: "Disabled",
    component: () => <Button disabled />,
  },
  {
    name: "Blur",
    component: () => <Button blur />,
  },
  {
    name: "Error",
    component: () => <Button error />,
  },
];

export default stories;
