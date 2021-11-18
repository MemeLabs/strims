import clsx from "clsx";
import React, { Suspense } from "react";
import { NavLink, Outlet } from "react-router-dom";

import LoadingPlaceholder from "../../root/LoadingPlaceholder";

const SettingsLayout: React.FC = () => {
  const linkClassName = ({ isActive }: { isActive: boolean }) =>
    clsx({
      "settings__nav__link": true,
      "settings__nav__link--active": isActive,
    });

  return (
    <div className="page_body settings">
      <div className="settings__nav">
        <NavLink className={linkClassName} to="/settings/networks">
          Networks
        </NavLink>
        <NavLink className={linkClassName} to="/settings/bootstrap-clients">
          Bootstrap Clients
        </NavLink>
        <NavLink className={linkClassName} to="/settings/chat-servers">
          Chat Servers
        </NavLink>
        <NavLink className={linkClassName} to="/settings/video-ingress">
          Video Ingress
        </NavLink>
        <NavLink className={linkClassName} to="/settings/vnic">
          VNIC
        </NavLink>
      </div>
      <div className="settings__body">
        <Suspense fallback={<LoadingPlaceholder />}>
          <Outlet />
        </Suspense>
      </div>
    </div>
  );
};

export default SettingsLayout;
