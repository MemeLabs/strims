import "./Footer.scss";

import React, { useCallback, useEffect, useLayoutEffect, useRef } from "react";
import { FiList, FiSearch, FiUser } from "react-icons/fi";
import { Link, Routes, useNavigate } from "react-router-dom";
import { useToggle } from "react-use";

import { useBackgroundRoute } from "../../contexts/BackgroundRoute";
import { settingsRoutes } from "../../root/MainRouter";
import Modal from "./Modal";

interface MemeModalProps {
  onClose: () => void;
}

const MemeModal1: React.FC<MemeModalProps> = ({ onClose }) => {
  const input = useRef<HTMLInputElement>(null);

  useLayoutEffect(() => input.current.focus(), []);

  return (
    <Modal title="Search" onClose={onClose}>
      <input ref={input} type="text" style={{ background: "white" }} />
    </Modal>
  );
};

const MemeModal2: React.FC<MemeModalProps> = ({ onClose }) => {
  const navigate = useNavigate();

  const backgroundRoute = useBackgroundRoute();
  console.log("===", backgroundRoute.location);
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
  const [showMemes1, toggleShowMemes1] = useToggle(false);
  const [showMemes2, toggleShowMemes2] = useToggle(false);

  return (
    <>
      {showMemes1 && <MemeModal1 onClose={() => toggleShowMemes1(false)} />}
      {showMemes2 && <MemeModal2 onClose={() => toggleShowMemes2(false)} />}
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
