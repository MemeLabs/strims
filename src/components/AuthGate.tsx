import React from "react";
import { Navigate, useLocation } from "react-router-dom";

import { useProfile } from "../contexts/Profile";

export interface AuthGateProps {
  children: React.ReactElement;
}

const AuthGate: React.FC<AuthGateProps> = ({ children }) => {
  const { pathname, search, hash } = useLocation();
  const [{ profile, loading }] = useProfile();

  if (loading) {
    return null;
  }
  if (profile) {
    return children;
  }

  const next = (pathname || "") + (search || "") + (hash || "");
  return <Navigate to={`/login?next=${encodeURIComponent(next)}`} />;
};

export default AuthGate;
