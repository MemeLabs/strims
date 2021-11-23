import "./Footer.scss";

import React, { RefObject, useCallback, useEffect, useRef } from "react";
import { FiList, FiMenu, FiSearch, FiUser } from "react-icons/fi";
import { useResolvedPath } from "react-router";
import { Link, Routes, useNavigate } from "react-router-dom";
import { useToggle } from "react-use";

import { useBackgroundRoute } from "../../contexts/BackgroundRoute";
import Layout, { Nav } from "../../pages/Settings/Layout";
import { createSettingsRoutes } from "../../root/MainRouter";
import Search from "../Directory/Search";
import Modal from "./Modal";

interface ModalProps {
  onClose: () => void;
}

interface SearchModalProps extends ModalProps {
  open: boolean;
  inputRef: RefObject<HTMLInputElement>;
}

const SearchModal: React.FC<SearchModalProps> = ({ open, onClose, inputRef }) => (
  <Modal className="footer__search" open={open} onClose={onClose}>
    <Search menuOpen={true} scrollMenu showCancel inputRef={inputRef} onDone={onClose} />
  </Modal>
);

const SettingsModal: React.FC<ModalProps> = ({ onClose }) => {
  const navigate = useNavigate();
  const backgroundRoute = useBackgroundRoute();

  const path = useResolvedPath("settings");

  useEffect(() => {
    const tid = setTimeout(() => {
      backgroundRoute.toggle(true);
      navigate(path);
    });
    return () => clearTimeout(tid);
  }, []);

  const handleModalClose = useCallback(() => {
    setTimeout(() => {
      backgroundRoute.toggle(false);
      onClose?.();
    });
  }, []);

  const [navOpen, toggleNavOpen] = useToggle(true);

  const title = (
    <div className="footer__settings__title">
      <button className="footer__settings__menu_button" onClick={() => toggleNavOpen()}>
        <FiMenu size={22} />
      </button>
      <div>Settings</div>
    </div>
  );

  return (
    <Modal title={title} onClose={handleModalClose}>
      <Routes>
        {createSettingsRoutes(<Layout nav={<Nav open={navOpen} onToggle={toggleNavOpen} />} />)}
      </Routes>
    </Modal>
  );
};

const Footer: React.FC = () => {
  const [showSearch, toggleShowSearch] = useToggle(false);
  const [showSettings, toggleShowSettings] = useToggle(false);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const handleSearchClick = () => {
    // if react defers rendering the search modal changes ios safari forgets
    // the event was triggered by a click and ignores the input focus call.
    searchInputRef.current.focus();
    toggleShowSearch(true);
  };

  return (
    <>
      <SearchModal
        inputRef={searchInputRef}
        open={showSearch}
        onClose={() => toggleShowSearch(false)}
      />
      {showSettings && <SettingsModal onClose={() => toggleShowSettings(false)} />}
      <div className="footer">
        <Link className="footer__button" to="/">
          <FiList className="footer__button__icon" />
        </Link>
        <button className="footer__button" onClick={handleSearchClick}>
          <FiSearch className="footer__button__icon" />
        </button>
        <button className="footer__button" onClick={toggleShowSettings}>
          <FiUser className="footer__button__icon" />
        </button>
      </div>
    </>
  );
};

export default Footer;
