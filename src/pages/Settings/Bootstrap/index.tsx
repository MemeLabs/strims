import React from "react";
import { Route, Routes } from "react-router-dom";

import BootstrapCreateForm from "./BootstrapCreateForm";
import BootstrapEditForm from "./BootstrapEditForm";
import BootstrapsList from "./BootstrapList";

const Router: React.FC = () => (
  <Routes>
    <Route index element={<BootstrapsList />} />
    <Route path="new" element={<BootstrapCreateForm />} />
    <Route path=":ruleId" element={<BootstrapEditForm />} />
  </Routes>
);

export default Router;
