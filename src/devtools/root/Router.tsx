import React, { lazy } from "react";
import { Route, Routes } from "react-router-dom";

import NotFound from "../pages/NotFound";

const Home = lazy(() => import("../pages/Home"));
const CapConn = lazy(() => import("../pages/CapConn"));
const Test = lazy(() => import("../pages/Test"));
const Emotes = lazy(() => import("../pages/Emotes"));
const Bridge = lazy(() => import("../pages/Bridge"));
const Layout = lazy(() => import("../pages/Layout"));
const Storybook = lazy(() => import("../pages/Storybook"));

const Router: React.FC = () => {
  return (
    <Routes>
      <Route path="/" element={<Home />} />
      <Route path="/capconn" element={<CapConn />} />
      <Route path="/test" element={<Test />} />
      <Route path="/emotes" element={<Emotes />} />
      <Route path="/bridge" element={<Bridge />} />
      <Route path="/layout" element={<Layout />} />
      <Route path="/storybook/*" element={<Storybook />} />
      <Route element={<NotFound />} />
    </Routes>
  );
};

export default Router;
