// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React from "react";
import { useAsync } from "react-use";

import Nav from "../components/Nav";
import { useCall, useClient } from "../contexts/DevToolsApi";

const Test: React.FC = () => {
  // declarative api
  const [testRes] = useCall("devTools", "test", { args: [{ name: "world" }] });

  // imperative api
  const client = useClient();
  const testRes2 = useAsync(() => client.devTools.test({ name: "world" }));

  return (
    <div>
      <Nav />
      <div>{testRes.value?.message}</div>
      <div>{testRes2.value?.message}</div>
    </div>
  );
};

export default Test;
