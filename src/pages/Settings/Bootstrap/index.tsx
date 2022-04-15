import React from "react";
import { Route, Routes } from "react-router-dom";

import BootstrapCreateForm from "./BootstrapCreateForm";
import BootstrapEditForm from "./BootstrapEditForm";
import BootstrapsList from "./BootstrapList";

const Router: React.FC = () => {
  return (
    <main className="network_page">
      <Routes>
        <Route index element={<BootstrapsList />} />
        <Route path="new" element={<BootstrapCreateForm />} />
        <Route path=":ruleId" element={<BootstrapEditForm />} />
      </Routes>
    </main>
  );
};

export default Router;
