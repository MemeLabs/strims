import * as React from "react";
import { Route, Switch } from "react-router-dom";

import { PrivateRoute } from "../components/PrivateRoute";
import Activity from "../pages/Activity";
import BootstrapClientsPage from "../pages/BootstrapClients";
import ChatServersPage from "../pages/ChatServers";
import ChatTest from "../pages/ChatTest";
import Directory from "../pages/Directory";
import HomePage from "../pages/Home";
import LoginPage from "../pages/Login";
import NetworkMembershipsPage from "../pages/NetworkMemberships";
import NetworksPage from "../pages/Networks";
import NotFoundPage from "../pages/NotFound";
import SignUpPage from "../pages/SignUp";

const Router = () => {
  return (
    <Switch>
      <Route path="/login" exact component={LoginPage} />
      <Route path="/signup" exact component={SignUpPage} />
      <PrivateRoute path="/" exact component={HomePage} />
      <PrivateRoute path="/networks" exact component={NetworksPage} />
      <PrivateRoute path="/memberships" exact component={NetworkMembershipsPage} />
      <PrivateRoute path="/bootstrap-clients" exact component={BootstrapClientsPage} />
      <PrivateRoute path="/chat-servers" exact component={ChatServersPage} />
      <PrivateRoute path="/directory/:networkKey" exact component={Directory} />
      <PrivateRoute path="/activity" exact component={Activity} />
      <PrivateRoute path="/chat-test" exact component={ChatTest} />
      <Route component={NotFoundPage} />
    </Switch>
  );
};

export default Router;
