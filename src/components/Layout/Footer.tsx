import "./Footer.scss";

import React, { useCallback, useEffect } from "react";
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

const SearchModal: React.FC<ModalProps> = ({ onClose }) => {
  const [open, toggleOpen] = useToggle(true);

  return (
    <Modal className="footer__search" open={open} onClose={onClose}>
      <Search menuOpen={true} scrollMenu autoFocus showCancel onDone={() => toggleOpen(false)} />
    </Modal>
  );
};

const SettingsModal: React.FC<ModalProps> = ({ onClose }) => {
  const navigate = useNavigate();

  const path = useResolvedPath("settings");

  const backgroundRoute = useBackgroundRoute();
  useEffect(() => {
    const tid = setTimeout(() => {
      backgroundRoute.toggle(true);
      navigate(path);
    });
    return () => clearTimeout(tid);
  }, []);

  const handleModalClose = useCallback(() => {
    const tid = setTimeout(() => {
      backgroundRoute.toggle(false);
      onClose?.();
    });
    return () => clearTimeout(tid);
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
  const [showSearch, toggleShowMemes1] = useToggle(false);
  const [showSettings, toggleShowMemes2] = useToggle(false);

  return (
    <>
      {showSearch && <SearchModal onClose={() => toggleShowMemes1(false)} />}
      {showSettings && <SettingsModal onClose={() => toggleShowMemes2(false)} />}
      <div className="footer">
        <Link className="footer__button" to="/">
          <FiList className="footer__button__icon" />
        </Link>
        <button className="footer__button" onClick={toggleShowMemes1}>
          <FiSearch className="footer__button__icon" />
        </button>
        <button className="footer__button" onClick={toggleShowMemes2}>
          <FiUser className="footer__button__icon" />
        </button>
      </div>
    </>
  );
};

export default Footer;
