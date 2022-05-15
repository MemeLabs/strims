// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route, Routes } from "react-router-dom";

import Home from "../pages/Home";
import NotFound from "../pages/NotFound";

const Router: React.FC = () => {
  return (
    <Routes>
      <Route path="/funding.html" element={<Home />} />
      <Route element={<NotFound />} />
    </Routes>
  );
};

export default Router;
