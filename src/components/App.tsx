import { EventEmitter } from "events";
import * as React from "react";
import * as api_pb from "../service/api_pb";

export interface Client {
  joinSwarm(v: api_pb.JoinSwarmRequest): Promise<api_pb.JoinSwarmResponse>;
  leaveSwarm(v: api_pb.LeaveSwarmRequest): Promise<api_pb.LeaveSwarmResponse>;
  getIngressStreams(v: api_pb.GetIngressStreamsRequest): EventEmitter;
}

const App = ({client}: {client: Client}) => {
  React.useEffect(() => {
    const streams = client.getIngressStreams(new api_pb.GetIngressStreamsRequest());
    streams.on("data", (stream: api_pb.GetIngressStreamsResponse) => {
      // console.log(stream.toObject());
    });
    streams.on("error", (v) => console.log(v));
  }, []);

  return (
    <div>
      <h1>bar</h1>
    </div>
  );
};

export default App;
