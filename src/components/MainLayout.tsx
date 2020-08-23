import clsx from "clsx";
import * as React from "react";
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

import { useTheme } from "../contexts/Theme";

const NetworkNav = () => {
  const [expanded, toggleExpanded] = useToggle(false);

  const classes = clsx({
    "main_layout__left": true,
    "main_layout__left--expanded": expanded,
    "main_layout__left--collapsed": !expanded,
  });

  const toggleIcon = expanded ? (
    <FiArrowLeft onClick={toggleExpanded} className="main_layout__left__toggle__icon" />
  ) : (
    <FiArrowRight onClick={toggleExpanded} className="main_layout__left__toggle__icon" />
  );

  return (
    <aside className={classes}>
      <button className="main_layout__left__toggle">
        <div className="main_layout__left__toggle__text">Networks</div>
        {toggleIcon}
      </button>
      <div className="main_layout__left__header_icon">
        <FiVideo />
      </div>
      <Link to="/" className="main_layout__left__gem">
        <FiBell />
      </Link>
      <Link to="/" className="main_layout__left__gem">
        <FiCloud />
      </Link>
      <Link to="/" className="main_layout__left__gem">
        <FiUser />
      </Link>
      <Link to="/networks" className="main_layout__left__gem">
        <FiPlusCircle />
      </Link>
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
          <Link to="/" className="main_layout__user_nav__link">
            <FiActivity />
          </Link>
          <Link to="/" className="main_layout__user_nav__link">
            <FiBell />
          </Link>
          <Link to="/" className="main_layout__user_nav__link">
            <FiCloud />
          </Link>
          <Link to="/" className="main_layout__user_nav__link">
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
