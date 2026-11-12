// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Navigation.scss";

import React, { ComponentPropsWithoutRef, useEffect } from "react";
import {
  Link as RouterLink,
  Navigate as RouterNavigate,
  useHref,
  useInRouterContext,
} from "react-router";

import {
  useInNavigationContext,
  useLinkClickHandler,
  useNavigation,
} from "../../contexts/SettingsNavigation";

interface LinkProps extends ComponentPropsWithoutRef<"a"> {
  to: string;
  back?: boolean;
  replace?: boolean;
}

// Whether the browser should handle a click on a link itself: a click
// other than a plain left one (a middle click, a modifier key held) or on a
// link opening elsewhere.
const browserHandlesClick = (e: React.MouseEvent<HTMLAnchorElement>, target?: string) =>
  e.button !== 0 ||
  e.metaKey ||
  e.altKey ||
  e.ctrlKey ||
  e.shiftKey ||
  (!!target && target !== "_self");

// A link in the settings navigation's context: a plain click pushes `to` to
// its history, others open its href (the router's for `to`, else `to`) as
// a link's would.
const NavigationLink: React.FC<LinkProps> = ({ to, back, replace, target, onClick, ...props }) => {
  const push = useLinkClickHandler(to, { back, replace });
  const href = useInRouterContext() ? useHref(to) : to;

  const handleClick = (e: React.MouseEvent<HTMLAnchorElement>) => {
    onClick?.(e);
    if (e.defaultPrevented || browserHandlesClick(e, target)) {
      return;
    }
    e.preventDefault();
    push();
  };

  return <a {...props} href={href} target={target} onClick={handleClick} />;
};

// The settings navigation's link where there is one, else the router's.
export const Link: React.FC<LinkProps> = ({ back, ...props }) =>
  useInNavigationContext() ? <NavigationLink back={back} {...props} /> : <RouterLink {...props} />;

export const Navigate: React.FC<LinkProps> = ({ to, replace }) => {
  const navigation = useNavigation();
  useEffect(() => navigation?.push(to, { replace }), [to, replace]);

  return navigation ? null : <RouterNavigate to={to} replace={replace} />;
};
