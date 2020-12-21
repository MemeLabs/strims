import * as React from "react";
import { Route, Switch } from "react-router-dom";

import Home from "../pages/Home";
import NotFound from "../pages/NotFound";

const Router = () => {
  return (
    <Switch>
      <Route path="/funding.html" exact component={Home} />
      <Route component={NotFound} />
    </Switch>
  );
};

export default Router;
