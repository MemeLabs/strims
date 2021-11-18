import React from "react";
import { useAsync } from "react-use";

import { useCall, useClient } from "../contexts/Api";

const Home: React.FC = () => {
  // declarative api
  const [testRes] = useCall("funding", "test", { args: [{ name: "world" }] });

  // imperative api
  const client = useClient();
  const testRes2 = useAsync(() => client.funding.test({ name: "world" }));

  return (
    <div>
      <div>{testRes.value?.message}</div>
      <div>{testRes2.value?.message}</div>
    </div>
  );
};

export default Home;
