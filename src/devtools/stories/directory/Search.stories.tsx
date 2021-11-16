import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerDirectoryFrontendService } from "../../../apis/strims/network/v1/directory/directory_rpc";
import Search from "../../../components/Directory/Search";
import { Provider as DirectoryProvider } from "../../../contexts/Directory";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import DirectoryService from "../../mocks/directory/service";

const Test: React.FC = () => {
  const [[service, client]] = React.useState((): [DirectoryService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new DirectoryService();
    registerDirectoryFrontendService(svc, service);

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  return (
    <div className="directory_mockup">
      <ApiProvider value={client}>
        <DirectoryProvider>
          <div className="directory_mockup__header">
            <Search />
          </div>
        </DirectoryProvider>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "Search",
    component: () => <Test />,
  },
];
