import React from "react";
import { Link, Switch } from "react-router-dom";

import { PrivateRoute } from "../../components/PrivateRoute";
import BootstrapClientsPage from "./BootstrapClients";
import ChatServersPage from "./ChatServers";
import NetworksPage from "./Networks";
import VideoIngressPage from "./VideoIngress";
import VNICPage from "./VNIC";

const SettingsPage = () => {
  return (
    <div className="page_body">
      <Link className="settings_link" to="/settings/networks">
        Networks
      </Link>
      <Link className="settings_link" to="/settings/bootstrap-clients">
        Bootstrap Clients
      </Link>
      <Link className="settings_link" to="/settings/chat-servers">
        Chat Servers
      </Link>
      <Link className="settings_link" to="/settings/video-ingress">
        Video Ingress
      </Link>
      <Link className="settings_link" to="/settings/vnic">
        VNIC
      </Link>
      <Switch>
        <PrivateRoute path="/settings/networks" exact component={NetworksPage} />
        <PrivateRoute path="/settings/bootstrap-clients" exact component={BootstrapClientsPage} />
        <PrivateRoute path="/settings/chat-servers" exact component={ChatServersPage} />
        <PrivateRoute path="/settings/video-ingress" component={VideoIngressPage} />
        <PrivateRoute path="/settings/vnic" component={VNICPage} />
      </Switch>
    </div>
  );
};

export default SettingsPage;
