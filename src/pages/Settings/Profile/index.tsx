// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route, Routes } from "react-router-dom";

import ProfileConfigForm from "./ProfileConfigForm";

const Router: React.FC = () => {
  return (
    <Routes>
      <Route index element={<ProfileConfigForm />} />
    </Routes>
  );
};

export default Router;