import * as React from "react";
import { Route, Switch } from "react-router-dom";

import { PrivateRoute } from "../components/PrivateRoute";
import Activity from "../pages/Activity";
import ChatTest from "../pages/ChatTest";
import Directory from "../pages/Directory";
import HomePage from "../pages/Home";
import LoginPage from "../pages/Login";
import NotFoundPage from "../pages/NotFound";
import PlayerTest from "../pages/PlayerTest";
import SettingsPage from "../pages/Settings";
import SignUpPage from "../pages/SignUp";

const Router = () => {
  return (
    <Switch>
      <Route path="/login" exact component={LoginPage} />
      <Route path="/signup" exact component={SignUpPage} />
      <PrivateRoute path="/" exact component={HomePage} />
      <PrivateRoute path="/settings" component={SettingsPage} />
      <PrivateRoute path="/directory/:networkKey" exact component={Directory} />
      <PrivateRoute path="/player/:networkKey" exact component={PlayerTest} />
      <PrivateRoute path="/activity" exact component={Activity} />
      <PrivateRoute path="/chat-test" exact component={ChatTest} />
      <Route component={NotFoundPage} />
    </Switch>
  );
};

export default Router;
