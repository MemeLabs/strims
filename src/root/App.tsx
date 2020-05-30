import * as React from "react";
import { useTitle } from "react-use";
import Client from "../lib/api/client";
import Provider from "./Provider";
import Router from "./Router";

const App = ({ client }: { client: Client }) => {
  // React.useEffect(() => {
  //   client.startHLSIngress(new pb.StartHLSIngressRequest());
  //   client.startHLSEgress(new pb.StartHLSEgressRequest());

  //   const streams = client.getIngressStreams(new pb.GetIngressStreamsRequest());
  //   streams.on("data", (stream) => {
  //     console.log(stream.toObject());
  //   });
  //   streams.on("error", (v) => console.log(v));
  // }, []);

  useTitle("Strims");

  return (
    <Provider client={client}>
      <Router />
    </Provider>
  );
};

export default App;
