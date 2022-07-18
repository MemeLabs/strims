// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { lazy } from "react";
import { Route, Routes } from "react-router-dom";

import AuthGate from "../components/AuthGate";
import { useBackgroundRoute } from "../contexts/BackgroundRoute";
import Login from "../pages/Login";
import SignUp from "../pages/SignUp";

const Main = lazy(() => import(/* webpackPrefetch: true */ "./Main"));

const RootRouter: React.FC = () => {
  const { location } = useBackgroundRoute();
  return (
    <Routes location={location}>
      <Route path="/login" element={<Login />} />
      <Route path="/signup" element={<SignUp />} />
      <Route
        path="/*"
        element={
          <AuthGate>
            <Main />
          </AuthGate>
        }
      />
    </Routes>
  );
};

export default RootRouter;
