// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";

import MainProvider from "./MainProvider";
import MainRouter from "./MainRouter";

const Main: React.FC = () => (
  <MainProvider>
    <MainRouter />
  </MainProvider>
);

export default Main;
