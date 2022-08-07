// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route, Routes } from "react-router-dom";

import DebugConfigForm from "./DebugConfigForm";
import DebugMockStream from "./DebugMockStream";

const Router: React.FC = () => {
  return (
    <Routes>
      <Route index element={<DebugConfigForm />} />
      <Route path="mock-stream" element={<DebugMockStream />} />
    </Routes>
  );
};

export default Router;
