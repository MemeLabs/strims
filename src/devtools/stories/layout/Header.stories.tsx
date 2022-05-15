// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "../../../components/Layout/Layout.scss";

import React from "react";

import Header from "../../../components/Layout/Header";
import { Provider as ThemeProvider } from "../../../contexts/Theme";

const Test: React.FC = () => (
  <ThemeProvider>
    <div className="layout layout--dark">
      <Header search={null} />
    </div>
  </ThemeProvider>
);

export default [
  {
    name: "header",
    component: () => <Test />,
  },
];
