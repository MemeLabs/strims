// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Navigation.scss";

import { useDrag } from "@use-gesture/react";
import clsx from "clsx";
import React, { ComponentProps, useRef } from "react";
import {
  Navigate as RouterNavigate,
  Routes,
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

export const PanelThing: React.FC = ({ children }) => {
  const navigation = useNavigation();

  const ref = useRef<HTMLDivElement>(null);
  useDrag(
    ({ movement: [mx], swipe: [sx], dragging }) => {
      console.log({ mx, sx, dragging });
      // let next: ContentState;
      // if (dragging) {
      //   if (showContent.closing) {
      //     setDragOffset(Math.max(mx, 0));
      //     next = { closed: false, closing: true, dragging: true };
      //   } else {
      //     setDragOffset(Math.max(-mx, 0));
      //     next = { closed: mx >= -10, closing: false, dragging: true };
      //   }
      // } else {
      //   const closed =
      //     (showContent.closing && (sx === 1 || mx > DRAG_THRESHOLD)) ||
      //     (!showContent.closing && sx !== -1 && mx > -DRAG_THRESHOLD);
      //   setDragOffset(0);
      //   next = { closed, closing: !closed, dragging: false };
      // }
      // if (!isEqual(showContent, next)) {
      //   setShowContent(next);
      // }
    },
    {
      target: ref,
      eventOptions: {
        capture: true,
        passive: false,
      },
    }
  );

  return (
    <div
      ref={ref}
      className={clsx({
        "panel_thing": true,
        [`panel_thing--focus_${navigation.focusedIndex}`]: true,
      })}
    >
      {navigation.history.map((location) => (
        <div className="pannel_thing__panel">
          <Routes location={location}>{children}</Routes>
        </div>
      ))}
    </div>
  );
};
