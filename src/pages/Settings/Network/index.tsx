// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { Route, Routes } from "react-router-dom";

import NetworkDirectoryEditForm from "./NetworkDirectoryEditForm";
import NetworkEditForm from "./NetworkEditForm";
import NetworkIconEditForm from "./NetworkIconEditForm";
import NetworkInviteCreateForm from "./NetworkInviteCreateForm";
import NetworkJoinForm from "./NetworkJoinForm";
import NetworkList from "./NetworkList";
import NetworkServerCreateForm from "./NetworkServerCreateForm";

const Router: React.FC = () => (
  <Routes>
    <Route index element={<NetworkList />} />
    <Route path="new" element={<NetworkServerCreateForm />} />
    <Route path="join" element={<NetworkJoinForm />} />
    <Route path=":networkId" element={<NetworkEditForm />} />
    <Route path=":networkId/directory" element={<NetworkDirectoryEditForm />} />
    <Route path=":networkId/icon" element={<NetworkIconEditForm />} />
    <Route path=":networkId/invite" element={<NetworkInviteCreateForm />} />
  </Routes>
);

export default Router;
