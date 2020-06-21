import * as React from "react";
import { Redirect, Route, RouteProps } from "react-router-dom";

import { useProfile } from "../contexts/Profile";

export const PrivateRoute = (props: RouteProps) => {
  const [{ profile, loading }] = useProfile();

  if (loading) {
    return <Route location={props.location} path={props.path} />;
  }
  if (profile) {
    return <Route {...props} />;
  }

  const paths = Array.isArray(props.path) ? props.path : [props.path];
  const redirects = paths.map((path) => (
    <Redirect key={path} path={path} to={`/login?next=${path}`} />
  ));
  return <>{redirects}</>;
};

PrivateRoute.displayName = "PrivateRoute";
