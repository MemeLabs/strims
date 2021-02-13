import React from "react";
import { useAsync } from "react-use";

import Nav from "../components/Nav";
import { useCall, useClient } from "../contexts/DevToolsApi";

const Test = () => {
  // declarative api
  const [testRes] = useCall("devTools", "test", { args: [{ name: "slugalisk" }] });

  // imperative api
  const client = useClient();
  const testRes2 = useAsync(() => client.devTools.test({ name: "slugalisk" }));

  return (
    <div>
      <Nav />
      <div>{testRes.value?.message}</div>
      <div>{testRes2.value?.message}</div>
    </div>
  );
};

export default Test;
