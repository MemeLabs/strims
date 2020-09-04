import clsx from "clsx";
import { Base64 } from "js-base64";
import Tooltip from "rc-tooltip";
import * as React from "react";
import { ReactElement } from "react";
import { BiNetworkChart } from "react-icons/bi";
import {
  FiActivity,
  FiArrowLeft,
  FiArrowRight,
  FiBell,
  FiCloud,
  FiHome,
  FiPlus,
  FiPlusCircle,
  FiSearch,
  FiUser,
  FiVideo,
} from "react-icons/fi";
import { Link } from "react-router-dom";
import { useToggle } from "react-use";

import { useCall, useClient } from "../contexts/Api";
import { useTheme } from "../contexts/Theme";

const NetworkNav = () => {
  const [expanded, toggleExpanded] = useToggle(false);
  const [networkMembershipsRes] = useCall("getNetworkMemberships");

  let links: ReactElement;
  if (networkMembershipsRes.error) {
    links = <div>error</div>;
  } else if (networkMembershipsRes.loading) {
    links = <div>...</div>;
  } else {
    links = (
      <>
        <Tooltip placement="right" overlay="Networks">
          <div className="main_layout__left__header_icon">
            <BiNetworkChart />
          </div>
        </Tooltip>
        {networkMembershipsRes.value.networkMemberships.map((membership) => (
          <Link
            to={`/directory/${Base64.fromUint8Array(membership.caCertificate.key, true)}`}
            className="main_layout__left__link"
            key={membership.id}
          >
            <div className="main_layout__left__link__gem">{membership.name.substr(0, 1)}</div>
            <div className="main_layout__left__link__text">{membership.name}</div>
          </Link>
        ))}
      </>
    );
  }

  const classes = clsx({
    "main_layout__left": true,
    "main_layout__left--expanded": expanded,
    "main_layout__left--collapsed": !expanded,
  });

  return (
    <aside className={classes}>
      <div className="main_layout__left__toggle">
        <div className="main_layout__left__toggle__text">Networks</div>
        <Tooltip
          placement="right"
          trigger={["hover", "click"]}
          overlay={expanded ? "Collapse" : "Expand"}
        >
          <button onClick={toggleExpanded} className="main_layout__left__toggle__icon">
            {expanded ? <FiArrowLeft /> : <FiArrowRight />}
          </button>
        </Tooltip>
      </div>
      {links}
      <button className="main_layout__left__add">
        <div className="main_layout__left__add__gem">
          <FiPlus />
        </div>
        <div className="main_layout__left__add__text">Add</div>
      </button>
    </aside>
  );
};

export const MainLayout = ({ children }: { children: any }) => {
  const [theme, { setColorScheme }] = useTheme();

  const toggleTheme = () =>
    theme.colorScheme === "dark" ? setColorScheme("light") : setColorScheme("dark");

  return (
    <div className="main_layout">
      <header className="main_layout__header">
        <div className="main_layout__primary_nav">
          <button onClick={toggleTheme} className="main_layout__primary_nav__logo">
            <FiHome />
          </button>
          <Link to="/networks" className="main_layout__primary_nav__link">
            Categories
          </Link>
          <Link to="/" className="main_layout__primary_nav__link">
            Streams
          </Link>
          <Link to="/broadcast" className="main_layout__primary_nav__link">
            Broadcast
          </Link>
        </div>
        <div className="main_layout__search">
          <input className="main_layout__search__input" placeholder="search..." />
          <button className="main_layout__search__button">
            <FiSearch />
          </button>
        </div>
        <div className="main_layout__user_nav">
          <Link to="/activity" className="main_layout__user_nav__link">
            <FiActivity />
          </Link>
          <Link to="/alerts" className="main_layout__user_nav__link">
            <FiBell />
          </Link>
          <Link to="/" className="main_layout__user_nav__link">
            <FiCloud />
          </Link>
          <Link to="/profile" className="main_layout__user_nav__link">
            <FiUser />
          </Link>
        </div>
      </header>
      <div className="main_layout__content">
        <NetworkNav />
        {children}
      </div>
    </div>
  );
};
