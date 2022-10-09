// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { lazy } from "react";
import { Route, Routes } from "react-router-dom";

import { useBackgroundRoute } from "../../contexts/BackgroundRoute";
import NotFound from "../pages/NotFound";

const Home = lazy(() => import("../pages/Home"));
const CapConn = lazy(() => import("../pages/CapConn"));
const Test = lazy(() => import("../pages/Test"));
const Emotes = lazy(() => import("../pages/Emotes"));
const Bridge = lazy(() => import("../pages/Bridge"));
const Layout = lazy(() => import("../pages/Layout"));
const Login = lazy(() => import("../pages/Login"));
const Storybook = lazy(() => import("../pages/Storybook"));

const Router: React.FC = () => {
  const { backgroundLocation } = useBackgroundRoute();
  return (
    <Routes location={backgroundLocation}>
      <Route index element={<Home />} />
      <Route path="/capconn" element={<CapConn />} />
      <Route path="/test" element={<Test />} />
      <Route path="/emotes" element={<Emotes />} />
      <Route path="/bridge" element={<Bridge />} />
      <Route path="/layout/*" element={<Layout />} />
      <Route path="/login/*" element={<Login />} />
      <Route path="/storybook/*" element={<Storybook />} />
      <Route path="*" element={<NotFound />} />
    </Routes>
  );
};

export default Router;
