import React from "react";
import { Route, Routes } from "react-router-dom";

import NetworkEditForm from "./NetworkEditForm";
import NetworkJoinForm from "./NetworkJoinForm";
import NetworkList from "./NetworkList";
import NetworkServerCreateForm from "./NetworkServerCreateForm";

const Router: React.FC = () => (
  <main className="network_page">
    <Routes>
      <Route index element={<NetworkList />} />
      <Route path="new" element={<NetworkServerCreateForm />} />
      <Route path="join" element={<NetworkJoinForm />} />
      <Route path=":networkId" element={<NetworkEditForm />} />
    </Routes>
  </main>
);

export default Router;
