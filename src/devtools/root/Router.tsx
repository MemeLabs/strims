import React, { lazy } from "react";
import { Route, Switch } from "react-router-dom";

import NotFound from "../pages/NotFound";

const Router: React.FC = () => {
  return (
    <Switch>
      <Route path="/" exact component={lazy(() => import("../pages/Home"))} />
      <Route path="/capconn" exact component={lazy(() => import("../pages/CapConn"))} />
      <Route path="/test" exact component={lazy(() => import("../pages/Test"))} />
      <Route path="/emotes" exact component={lazy(() => import("../pages/Emotes"))} />
      <Route path="/bridge" exact component={lazy(() => import("../pages/Bridge"))} />
      <Route path="/layout" component={lazy(() => import("../pages/Layout"))} />
      <Route path="/storybook" component={lazy(() => import("../pages/Storybook"))} />
      <Route component={NotFound} />
    </Switch>
  );
};

export default Router;
