import React, { Suspense, lazy } from "react";
import { Redirect, Route, Switch } from "react-router-dom";

import { MainBodyLayout, MainLayout } from "../components/MainLayout";
import { PrivateRoute } from "../components/PrivateRoute";
import Login from "../pages/Login";
import NotFound from "../pages/NotFound";
import SettingsLayout from "../pages/Settings/Layout";
import SignUp from "../pages/SignUp";

const SettingsRouter: React.FC = () => (
  <Suspense fallback={null}>
    <Switch>
      <PrivateRoute
        path="/settings/networks"
        exact
        component={lazy(() => import("../pages/Settings/Networks"))}
      />
      <PrivateRoute
        path="/settings/bootstrap-clients"
        exact
        component={lazy(() => import("../pages/Settings/BootstrapClients"))}
      />
      <PrivateRoute
        path="/settings/chat-servers"
        component={lazy(() => import("../pages/Settings/ChatServers"))}
      />
      <PrivateRoute
        path="/settings/video-ingress"
        component={lazy(() => import("../pages/Settings/VideoIngress"))}
      />
      <PrivateRoute
        path="/settings/vnic"
        component={lazy(() => import("../pages/Settings/VNIC"))}
      />
      <Redirect to="/settings/networks" />
    </Switch>
  </Suspense>
);

const MainRouter: React.FC = () => (
  <Switch>
    <PrivateRoute path="/settings">
      <SettingsLayout>
        <SettingsRouter />
      </SettingsLayout>
    </PrivateRoute>
    <PrivateRoute>
      <MainBodyLayout>
        <Suspense fallback={null}>
          <Switch>
            <PrivateRoute path="/" exact component={lazy(() => import("../pages/Home"))} />
            <PrivateRoute
              path="/directory/:networkKey"
              exact
              component={lazy(() => import("../pages/Directory"))}
            />
            <PrivateRoute
              path="/player/:networkKey"
              exact
              component={lazy(() => import("../pages/PlayerTest"))}
            />
            <PrivateRoute
              path="/activity"
              exact
              component={lazy(() => import("../pages/Activity"))}
            />
            <PrivateRoute
              path="/chat-test"
              exact
              component={lazy(() => import("../pages/ChatTest"))}
            />
            <Redirect to="/404" />
          </Switch>
        </Suspense>
      </MainBodyLayout>
    </PrivateRoute>
  </Switch>
);

const RootRouter: React.FC = () => (
  <Switch>
    <Route path="/login" exact component={Login} />
    <Route path="/signup" exact component={SignUp} />
    <Route path="/404" exact component={NotFound} />
    <PrivateRoute>
      <MainLayout>
        <MainRouter />
      </MainLayout>
    </PrivateRoute>
  </Switch>
);

export default RootRouter;
