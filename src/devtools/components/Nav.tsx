import React from "react";
import { Link } from "react-router-dom";

const Nav: React.FC = () => (
  <div className="nav">
    <Link to="/">Home</Link>
    <Link to="/test">Test</Link>
    <Link to="/emotes">Emotes</Link>
  </div>
);

export default Nav;
