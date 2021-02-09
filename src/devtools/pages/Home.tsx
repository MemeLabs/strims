import * as React from "react";
import { useAsync } from "react-use";

import { CapConnWatchLogsResponse } from "../../apis/strims/devtools/v1/ppspp/capconn";
import Nav from "../components/Nav";
import { useCall, useClient } from "../contexts/DevToolsApi";

const reduceLogs = (logs: string[], event: CapConnWatchLogsResponse): string[] => {
  switch (event.op) {
    case CapConnWatchLogsResponse.Op.CREATE:
      return [...logs, event.name].sort();
    case CapConnWatchLogsResponse.Op.REMOVE:
      return logs.filter((n) => n !== event.name);
  }
};

const useLogs = () => {
  const client = useClient();
  const [logs, dispatch] = React.useReducer(reduceLogs, []);

  React.useEffect(() => {
    const events = client.ppsppCapConn.watchLogs();
    events.on("data", dispatch);
    return () => events.destroy();
  }, []);

  return logs;
};

const Files = ({ onSelect }: { onSelect: (string) => void }) => {
  const logs = useLogs();

  return (
    <ul className="network_list">
      {logs.map((name, i) => (
        <li key={i} onClick={() => onSelect(name)}>
          {name}
        </li>
      ))}
    </ul>
  );
};

const Home = () => {
  const client = useClient();
  const handleFileSelect = async (name: string) => {
    const log = await client.ppsppCapConn.loadLog({ name });
    console.log(log);
  };

  return (
    <div>
      <Nav />
      <Files onSelect={handleFileSelect} />
    </div>
  );
};

export default Home;
