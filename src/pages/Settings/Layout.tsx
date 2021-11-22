import "./Layout.scss";

import clsx from "clsx";
import React, { ReactElement, Suspense } from "react";
import { NavLink, Outlet } from "react-router-dom";

import SwipablePanel from "../../components/SwipablePanel";
import LoadingPlaceholder from "../../root/LoadingPlaceholder";

interface NavProps {
  open?: boolean;
  onToggle?: (value: boolean) => void;
}

export const Nav: React.FC<NavProps> = ({ open, onToggle }) => {
  const linkClassName = ({ isActive }: { isActive: boolean }) =>
    clsx({
      "settings__nav__link": true,
      "settings__nav__link--active": isActive,
    });

  return (
    <SwipablePanel className="settings__nav" open={open} onToggle={onToggle} direction="right">
      <NavLink className={linkClassName} to="networks">
        Networks
      </NavLink>
      <NavLink className={linkClassName} to="bootstrap-clients">
        Bootstrap Clients
      </NavLink>
      <NavLink className={linkClassName} to="chat-servers">
        Chat Servers
      </NavLink>
      <NavLink className={linkClassName} to="video-ingress">
        Video Ingress
      </NavLink>
      <NavLink className={linkClassName} to="vnic">
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
        <Outlet />
      </Suspense>
    </div>
  </div>
);

export default SettingsLayout;
