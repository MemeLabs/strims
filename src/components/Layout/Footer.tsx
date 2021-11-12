import "./Footer.scss";

import React from "react";
import { FiList, FiSearch, FiUser } from "react-icons/fi";
import { Link } from "react-router-dom";

const Footer: React.FC = () => (
  <div className="footer">
    <Link className="footer__button" to="/">
      <FiList className="footer__button__icon" />
    </Link>
    <button className="footer__button">
      <FiSearch className="footer__button__icon" />
    </button>
    <Link className="footer__button" to="/settings">
      <FiUser className="footer__button__icon" />
    </Link>
  </div>
);

export default Footer;
