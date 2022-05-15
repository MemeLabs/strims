// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./Layout.scss";

import clsx from "clsx";
import React, { ReactElement, Suspense, useCallback } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { NavLink, Outlet } from "react-router-dom";

import LoadingPlaceholder from "../../components/LoadingPlaceholder";
import SwipablePanel from "../../components/SwipablePanel";
import { useCall } from "../../contexts/FrontendApi";

export interface NavProps {
  open?: boolean;
  onToggle?: (value?: boolean) => void;
}

export const Nav: React.FC<NavProps> = ({ open, onToggle }) => {
  const [videoIngressIsSupported] = useCall("videoIngress", "isSupported");
  const [hlsEgressIsSupported] = useCall("hlsEgress", "isSupported");

  const linkClassName = ({ isActive }: { isActive: boolean }) =>
    clsx({
      "settings__nav__link": true,
      "settings__nav__link--active": isActive,
    });

  const onLinkClick = useCallback(() => onToggle?.(), []);

  if (videoIngressIsSupported.loading || hlsEgressIsSupported.loading) {
    return null;
  }

  return (
    <SwipablePanel
      className="settings__nav"
      open={open}
      onToggle={onToggle}
      dragThreshold={100}
      direction="right"
    >
      <NavLink className={linkClassName} onClick={onLinkClick} to="networks">
        Networks
      </NavLink>
      <NavLink className={linkClassName} onClick={onLinkClick} to="bootstraps">
        Bootstraps
      </NavLink>
      <NavLink className={linkClassName} onClick={onLinkClick} to="chat-servers">
        Chat Servers
      </NavLink>
      {videoIngressIsSupported.value.supported && (
        <NavLink className={linkClassName} onClick={onLinkClick} to="video/ingress">
          Video Ingress
        </NavLink>
      )}
      {hlsEgressIsSupported.value.supported && (
        <NavLink className={linkClassName} onClick={onLinkClick} to="video/egress">
          Video Egress
        </NavLink>
      )}
      <NavLink className={linkClassName} onClick={onLinkClick} to="autoseed/config">
        Autoseed
      </NavLink>
      <NavLink className={linkClassName} onClick={onLinkClick} to="vnic">
        VNIC
      </NavLink>
    </SwipablePanel>
  );
};

interface SettingsLayoutProps {
  nav?: ReactElement;
}

const SettingsLayout: React.FC<SettingsLayoutProps> = ({ nav = <Nav /> }) => (
  <div className="settings">
    {nav}
    <div className="settings__body">
      <Suspense fallback={<LoadingPlaceholder />}>
        <Scrollbars autoHide={true}>
          <main className="settings__content">
            <Outlet />
          </main>
        </Scrollbars>
      </Suspense>
    </div>
  </div>
);

export default SettingsLayout;
