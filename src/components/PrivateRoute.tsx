import React from "react";
import { Redirect, Route, RouteProps } from "react-router-dom";

import { useProfile } from "../contexts/Profile";

export const PrivateRoute: React.FC<RouteProps> = (props) => {
  const [{ profile, loading }] = useProfile();

  if (loading) {
    return <Route location={props.location} path={props.path} />;
  }
  if (profile) {
    return <Route {...props} />;
  }

  return (
    <Route
      location={props.location}
      path={props.path}
      render={({ location: { pathname, search, hash } }) => {
        const next = (pathname || "") + (search || "") + (hash || "");
        return <Redirect to={`/login?next=${encodeURIComponent(next)}`} />;
      }}
    />
  );
};

PrivateRoute.displayName = "PrivateRoute";
