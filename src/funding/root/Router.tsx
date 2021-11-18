import React from "react";
import { Route, Routes } from "react-router-dom";

import Home from "../pages/Home";
import NotFound from "../pages/NotFound";

const Router: React.FC = () => {
  return (
    <Routes>
      <Route path="/funding.html" element={<Home />} />
      <Route element={<NotFound />} />
    </Routes>
  );
};

export default Router;
