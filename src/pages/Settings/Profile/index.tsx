// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route, Routes } from "react-router-dom";

import ProfileConfigForm from "./ProfileConfigForm";
import ProfileDeviceList from "./ProfileDeviceList";

const Router: React.FC = () => {
  return (
    <Routes>
      <Route index element={<ProfileConfigForm />} />
      <Route path="devices" element={<ProfileDeviceList />} />
    </Routes>
  );
};

export default Router;
