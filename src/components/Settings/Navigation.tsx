// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Navigation.scss";

import React, { ComponentProps } from "react";
import {
  Navigate as RouterNavigate,
  useHref,
  useLinkClickHandler as useRouterLinkClickHandler,
} from "react-router-dom";

import {
  useInNavigationContext,
  useLinkClickHandler,
  useNavigation,
} from "../../contexts/SettingsNavigation";

interface LinkProps extends ComponentProps<"a"> {
  to: string;
  back?: boolean;
  replace?: boolean;
}

export const Link: React.FC<LinkProps> = ({ to, back, replace, ...props }) => {
  const handleClick = useInNavigationContext()
    ? useLinkClickHandler(to, { back, replace })
    : useRouterLinkClickHandler(useHref(to));

  return <a {...props} onClick={handleClick} />;
};

export const Navigate: React.FC<LinkProps> = ({ to, replace }) => {
  if (useInNavigationContext()) {
    useNavigation().push(to, { replace });
    return null;
  }

  return <RouterNavigate to={to} replace={replace} />;
};
