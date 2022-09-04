// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route, Routes } from "react-router-dom";

import BootstrapClientList from "./BootstrapClientList";
import BootstrapCreateClientForm from "./BootstrapCreateClientForm";
import BootstrapEditClientForm from "./BootstrapEditClientForm";
import BotostrapConfigForm from "./BotostrapConfigForm";

const Router: React.FC = () => (
  <Routes>
    <Route index element={<BotostrapConfigForm />} />
    <Route path="clients" element={<BootstrapClientList />} />
    <Route path="clients/new" element={<BootstrapCreateClientForm />} />
    <Route path="clients/:clientId" element={<BootstrapEditClientForm />} />
  </Routes>
);

export default Router;
