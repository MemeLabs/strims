import "./LoadingPlaceholder.scss";

import clsx from "clsx";
import React from "react";

import { withTheme } from "./Theme";

interface LoadingPlaceholderProps {
  className: string;
}

const LoadingPlaceholder: React.FC<LoadingPlaceholderProps> = ({ className }) => (
  <div className={clsx(className, "loading_placeholder")}>loading</div>
);

export default withTheme(LoadingPlaceholder);
