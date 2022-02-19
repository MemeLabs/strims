import "../../../components/Layout/Layout.scss";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerNetworkServiceService } from "../../../apis/strims/network/v1/network_rpc";
import NetworkNav from "../../../components/Layout/NetworkNav";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { withLayoutContext } from "../../../contexts/Layout";
import { Provider as NetworkProvider } from "../../../contexts/Network";
import { AsyncPassThrough } from "../../../lib/stream";
import NetworkService from "../../mocks/network/service";

const NavTest = withLayoutContext(({ rootRef }) => (
  <div ref={rootRef} className="layout layout--dark">
    <div
      style={{
        "display": "flex",
        "flexDirection": "row",
        "height": "100%",
      }}
    >
      <NetworkNav />
    </div>
  </div>
));

const Test: React.FC = () => {
  const [[service, client]] = React.useState((): [NetworkService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new NetworkService();
    registerNetworkServiceService(svc, service);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  return (
    <ApiProvider value={client}>
      <NetworkProvider>
        <NavTest />
      </NetworkProvider>
    </ApiProvider>
  );
};

export default [
  {
    name: "nav",
    component: () => <Test />,
  },
];
