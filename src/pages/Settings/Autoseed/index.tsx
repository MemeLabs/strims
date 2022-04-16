import React from "react";
import { Route, Routes } from "react-router-dom";

import AutoseedConfigForm from "./AutoseedConfigForm";
import AutoseedRuleCreateForm from "./AutoseedRuleCreateForm";
import AutoseedRuleEditForm from "./AutoseedRuleEditForm";
import AutoseedRulesList from "./AutoseedRulesList";

const Router: React.FC = () => {
  return (
    <Routes>
      <Route path="config" element={<AutoseedConfigForm />} />
      <Route path="rules" element={<AutoseedRulesList />} />
      <Route path="rules/new" element={<AutoseedRuleCreateForm />} />
      <Route path="rules/:ruleId" element={<AutoseedRuleEditForm />} />
    </Routes>
  );
};

export default Router;
