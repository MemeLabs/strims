import "./Theme.scss";

import clsx from "clsx";
import React, { ComponentType } from "react";

import { useTheme } from "../contexts/Theme";

export interface WithThemeProps {
  className: string;
}

export const withTheme = <T,>(
  C: ComponentType<T & WithThemeProps>
): React.FC<Omit<T, keyof WithThemeProps>> => {
  const Theme: React.FC<T> = (props) => {
    const { colorScheme } = useTheme();

    return <C {...props} className={clsx("theme", `theme--${colorScheme}`)} />;
  };

  Theme.displayName = "Theme";

  return Theme;
};
