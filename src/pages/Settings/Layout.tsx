import "./Layout.scss";

import clsx from "clsx";
import React, { ReactElement, Suspense, useCallback } from "react";
import Scrollbars from "react-custom-scrollbars-2";
import { NavLink, Outlet } from "react-router-dom";

import SwipablePanel from "../../components/SwipablePanel";
import LoadingPlaceholder from "../../root/LoadingPlaceholder";

export interface NavProps {
  open?: boolean;
  onToggle?: (value?: boolean) => void;
}

export const Nav: React.FC<NavProps> = ({ open, onToggle }) => {
  const linkClassName = ({ isActive }: { isActive: boolean }) =>
    clsx({
      "settings__nav__link": true,
      "settings__nav__link--active": isActive,
    });

  const onLinkClick = useCallback(() => onToggle?.(), []);

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
      <NavLink className={linkClassName} onClick={onLinkClick} to="bootstrap-clients">
        Bootstrap Clients
      </NavLink>
      <NavLink className={linkClassName} onClick={onLinkClick} to="chat-servers">
        Chat Servers
      </NavLink>
      <NavLink className={linkClassName} onClick={onLinkClick} to="video">
        Video Ingress
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
          <Outlet />
        </Scrollbars>
      </Suspense>
    </div>
  </div>
);

export default SettingsLayout;
