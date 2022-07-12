// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./LandingPageLayout.scss";

import clsx from "clsx";
import React, { ReactNode } from "react";

import { withTheme } from "./Theme";

interface LandingPageLayoutProps {
  className: string;
  children: ReactNode;
}

const LandingPageLayout: React.FC<LandingPageLayoutProps> = ({ className, children }) => (
  <div className={clsx(className, "landing_page")}>
    <div className="landing_page__body">
      <div className="landing_page__header">
        <h1 className="landing_page__header__title">strims@home</h1>
        <span className="landing_page__header__tagline">Watch strims with frens.</span>
      </div>
      <div className="landing_page__form_container">
        {children}
        {/* <button
            onClick={() => theme.setColorScheme(theme.colorScheme === "dark" ? "light" : "dark")}
          >
            theme
          </button> */}
      </div>
    </div>
  </div>
);

export default withTheme(LandingPageLayout);
