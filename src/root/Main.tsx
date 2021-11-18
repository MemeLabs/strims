import React from "react";

import MainProvider from "./MainProvider";
import MainRouter from "./MainRouter";

const Main: React.FC = () => (
  <MainProvider>
    <MainRouter />
  </MainProvider>
);

export default Main;
