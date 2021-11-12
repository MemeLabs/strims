import React from "react";
import { Switch } from "react-router-dom";

import { PrivateRoute } from "../../../components/PrivateRoute";
import NetworkEditForm from "./NetworkEditForm";
import NetworkJoinForm from "./NetworkJoinForm";
import NetworkList from "./NetworkList";
import NetworkServerCreateForm from "./NetworkServerCreateForm";

const Router: React.FC = () => (
  <main className="network_page">
    <Switch>
      <PrivateRoute path="/settings/networks" exact component={NetworkList} />
      <PrivateRoute path="/settings/networks/new" exact component={NetworkServerCreateForm} />
      <PrivateRoute path="/settings/networks/join" exact component={NetworkJoinForm} />
      <PrivateRoute path="/settings/networks/:networkId" exact component={NetworkEditForm} />
    </Switch>
  </main>
);

export default Router;
