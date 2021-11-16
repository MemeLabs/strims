import "./Header.scss";

import React, { useCallback, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { FiActivity, FiBell, FiCloud, FiSearch, FiUser } from "react-icons/fi";
import { MdOutlineChat, MdOutlineChatBubbleOutline } from "react-icons/md";
import { Link, NavLink } from "react-router-dom";

import { MetricsFormat } from "../../apis/strims/debug/v1/debug";
import { useClient } from "../../contexts/FrontendApi";
import { useLayout } from "../../contexts/Layout";
import Debugger from "../Debugger";
import DirectorySearch from "../Directory/Search";

const Header: React.FC = () => {
  const { t } = useTranslation();
  const client = useClient();
  const layout = useLayout();

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
        {/* <input className="layout_header__search__input" placeholder={t("layout.header.Search")} />
        <button className="layout_header__search__button">
          <FiSearch />
        </button> */}
        <DirectorySearch />
      </div>
      <div className="layout_header__user_nav">
        <button className="layout_header__user_nav__link" onClick={() => layout.toggleShowChat()}>
          {layout.showChat ? (
            <MdOutlineChatBubbleOutline title={t("layout.header.Close chat")} />
          ) : (
            <MdOutlineChat title={t("layout.header.Open chat")} />
          )}
        </button>
        <button
          onClick={handleDebuggerOpen}
          className="layout_header__user_nav__link"
          title={t("layout.header.Activity monitor")}
        >
          <FiActivity />
        </button>
        <button className="layout_header__user_nav__link">
          <FiCloud />
        </button>
        <button onClick={handleAlertsClick} className="layout_header__user_nav__link">
          <FiBell />
        </button>
        <Debugger isOpen={debuggerIsOpen} onClose={handleDebuggerClose} />
        <Link
          className="layout_header__user_nav__link"
          to="/settings"
          title={t("layout.header.Settings")}
        >
          <FiUser />
        </Link>
      </div>
    </header>
  );
};

export default Header;
