import React from "react";
import { Route, Switch } from "react-router-dom";

import Home from "../pages/Home";
import NotFound from "../pages/NotFound";
import Test from "../pages/Test";
import Emotes from "../pages/Emotes";

const Router = () => {
  return (
    <Switch>
      <Route path="/" exact component={Home} />
      <Route path="/test" exact component={Test} />
      <Route path="/emotes" exact component={Emotes} />
      <Route component={NotFound} />
    </Switch>
  );
};

export default Router;
