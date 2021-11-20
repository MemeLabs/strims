import "./Footer.scss";

import React, { useCallback, useEffect } from "react";
import { FiList, FiSearch, FiUser } from "react-icons/fi";
import { Link, Routes, useNavigate } from "react-router-dom";
import { useToggle } from "react-use";

import { useBackgroundRoute } from "../../contexts/BackgroundRoute";
import { settingsRoutes } from "../../root/MainRouter";
import Search from "../Directory/Search";
import Modal from "./Modal";

interface ModalProps {
  onClose: () => void;
}

const SearchModal: React.FC<ModalProps> = ({ onClose }) => {
  const [open, toggleOpen] = useToggle(true);

  return (
    <Modal className="layout_footer__search" open={open} onClose={onClose}>
      <Search menuOpen={true} scrollMenu autoFocus showCancel onDone={() => toggleOpen(false)} />
    </Modal>
  );
};

const SettingsModal: React.FC<ModalProps> = ({ onClose }) => {
  const navigate = useNavigate();

  const backgroundRoute = useBackgroundRoute();
  useEffect(() => {
    backgroundRoute.toggle(true);
    navigate("/settings");
  }, []);

  const handleModalClose = useCallback(() => {
    backgroundRoute.toggle(false);
    onClose?.();
  }, []);

  return (
    <Modal title="Settings" onClose={handleModalClose}>
      <Routes>{settingsRoutes}</Routes>
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
