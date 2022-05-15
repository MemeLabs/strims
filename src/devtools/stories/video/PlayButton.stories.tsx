// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import LogoButton from "../../../components/VideoPlayer/LogoButton";

export type ButtonProps = {
  spin?: boolean;
  flicker?: boolean;
  pulse?: boolean;
  disabled?: boolean;
  visible?: boolean;
  blur?: boolean;
  error?: boolean;
};

const Button: React.FC<ButtonProps> = (props) => (
  <div className="combo app app--dark">
    <LogoButton {...props} onClick={() => null} />
  </div>
);

export default [
  {
    name: "spin",
    component: () => <Button spin />,
  },
  {
    name: "flicker",
    component: () => <Button flicker />,
  },
  {
    name: "pulse",
    component: () => <Button pulse />,
  },
  {
    name: "disabled",
    component: () => <Button disabled />,
  },
  {
    name: "blur",
    component: () => <Button blur />,
  },
  {
    name: "error",
    component: () => <Button error />,
  },
];
