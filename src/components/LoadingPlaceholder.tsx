// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./LoadingPlaceholder.scss";

import clsx from "clsx";
import React from "react";

import { InputError } from "./Form";
import { withTheme } from "./Theme";

interface LoadingPlaceholderProps {
  className: string;
  errorNode: boolean;
}

const LoadingPlaceholder: React.FC<LoadingPlaceholderProps> = ({
  className,
  errorNode = false,
}) => (
  <>
    {errorNode && (
      <div className={clsx(className, "loading_placeholder")}>
        <InputError error={"Error Establishing a Database Connection"} />
      </div>
    )}
    {!errorNode && <div className={clsx(className, "loading_placeholder")}>loading</div>}
  </>
);

export default withTheme(LoadingPlaceholder);
