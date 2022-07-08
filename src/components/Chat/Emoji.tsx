// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Emoji.scss";

import clsx from "clsx";
import React from "react";

export type EmojiProps = React.ComponentProps<"span">;

const Emoji: React.FC<EmojiProps> = ({ children, className, ...props }) => {
  return (
    <span className={clsx("emoji", className)} {...props}>
      {children}
    </span>
  );
};

export default Emoji;
