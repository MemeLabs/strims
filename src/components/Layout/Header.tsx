import "./Header.scss";

import React, { useCallback, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { FiActivity, FiBell, FiCloud, FiSearch, FiUser } from "react-icons/fi";
import { Link, NavLink } from "react-router-dom";

import { MetricsFormat } from "../../apis/strims/debug/v1/debug";
import { useClient } from "../../contexts/FrontendApi";
import { useTheme } from "../../contexts/Theme";
import Debugger from "../Debugger";

const Header: React.FC = () => {
  const { t } = useTranslation();
  const client = useClient();

  const [debuggerIsOpen, setDebuggerIsOpen] = useState(false);
  const handleDebuggerClose = useCallback(() => setDebuggerIsOpen(false), []);
  const handleDebuggerOpen = () => setDebuggerIsOpen(true);

  const handleAlertsClick = async () => {
    const { data } = await client.debug.readMetrics({
      format: MetricsFormat.METRICS_FORMAT_TEXT,
    });
    console.log(new TextDecoder().decode(data));
  };

  return (
    <header className="layout_header">
      <div className="layout_header__primary_nav">
        <Link to="/" className="layout_header__primary_nav__logo">
          <Trans>layout.header.Home</Trans>
        </Link>
        <NavLink
          to="/settings"
          className="layout_header__primary_nav__link"
          activeClassName="layout_header__primary_nav__link--active"
        >
          <Trans>layout.header.Categories</Trans>
        </NavLink>
        <NavLink
          to="/"
          exact
          className="layout_header__primary_nav__link"
          activeClassName="layout_header__primary_nav__link--active"
        >
          <Trans>layout.header.Streams</Trans>
        </NavLink>
        <NavLink
          to="/broadcast"
          className="layout_header__primary_nav__link"
          activeClassName="layout_header__primary_nav__link--active"
        >
          <Trans>layout.header.Broadcast</Trans>
        </NavLink>
      </div>
      <div className="layout_header__search">
        <input className="layout_header__search__input" placeholder={t("layout.header.Search")} />
        <button className="layout_header__search__button">
          <FiSearch />
        </button>
      </div>
      <div className="layout_header__user_nav">
        <button className="layout_header__user_nav__link">
          <FiActivity />
        </button>
        <button onClick={handleAlertsClick} className="layout_header__user_nav__link">
          <FiBell />
        </button>
        <button onClick={handleDebuggerOpen} className="layout_header__user_nav__link">
          <FiCloud />
          <Debugger isOpen={debuggerIsOpen} onClose={handleDebuggerClose} />
        </button>
        <button className="layout_header__user_nav__link">
          <FiUser />
        </button>
      </div>
    </header>
  );
};

export default Header;
