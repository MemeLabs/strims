import React from "react";
import { NavLink } from "react-router-dom";

const SettingsLayout: React.FC = ({ children }) => {
  return (
    <div className="page_body settings">
      <div className="settings__nav">
        <NavLink
          className="settings__nav__link"
          activeClassName="settings__nav__link--active"
          to="/settings/networks"
        >
          Networks
        </NavLink>
        <NavLink
          className="settings__nav__link"
          activeClassName="settings__nav__link--active"
          to="/settings/bootstrap-clients"
        >
          Bootstrap Clients
        </NavLink>
        <NavLink
          className="settings__nav__link"
          activeClassName="settings__nav__link--active"
          to="/settings/chat-servers"
        >
          Chat Servers
        </NavLink>
        <NavLink
          className="settings__nav__link"
          activeClassName="settings__nav__link--active"
          to="/settings/video-ingress"
        >
          Video Ingress
        </NavLink>
        <NavLink
          className="settings__nav__link"
          activeClassName="settings__nav__link--active"
          to="/settings/vnic"
        >
          VNIC
        </NavLink>
      </div>
      <div className="settings__body">{children}</div>
    </div>
  );
};

export default SettingsLayout;
