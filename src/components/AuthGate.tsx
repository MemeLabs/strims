// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Navigate, useLocation } from "react-router-dom";

import { useSession } from "../contexts/Session";

export interface AuthGateProps {
  children: React.ReactElement;
}

const AuthGate: React.FC<AuthGateProps> = ({ children }) => {
  const { pathname, search, hash } = useLocation();
  const [{ linkedProfiles, profile, loading }] = useSession();

  if (loading) {
    return null;
  }
  if (profile) {
    return children;
  }

  const page = linkedProfiles.length ? "login" : "signup";
  const next = (pathname || "") + (search || "") + (hash || "");
  return <Navigate to={`/${page}?next=${encodeURIComponent(next)}`} />;
};

export default AuthGate;
