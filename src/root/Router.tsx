// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { lazy } from "react";
import { Route, Routes } from "react-router-dom";

import AuthGate from "../components/AuthGate";
import { useBackgroundRoute } from "../contexts/BackgroundRoute";
import Login from "../pages/Login";
import SignUp from "../pages/SignUp";

const Main = lazy(() => import("./Main"));

const RootRouter: React.FC = () => {
  const { backgroundLocation } = useBackgroundRoute();
  return (
    <Routes location={backgroundLocation}>
      <Route path="/login" element={<Login />} />
      <Route path="/login/new" element={<Login newLogin />} />
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
