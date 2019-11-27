import * as React from "react";
import * as api_pb from "../service/api_pb";
import Client from "../service/client";

const App = ({client}: {client: Client}) => {
  React.useEffect(() => {
    // client.startHLSIngress(new api_pb.StartHLSIngressRequest());
    // client.startHLSEgress(new api_pb.StartHLSEgressRequest());

    // const streams = client.getIngressStreams(new api_pb.GetIngressStreamsRequest());
    // streams.on("data", (stream: api_pb.GetIngressStreamsResponse) => {
    //   // console.log(stream.toObject());
    // });
    // streams.on("error", (v) => console.log(v));
  }, []);

  return (
    <div>
      <h1>bar</h1>
    </div>
  );
};

export default App;
