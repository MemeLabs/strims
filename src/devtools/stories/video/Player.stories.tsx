// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerEgressService } from "../../../apis/strims/video/v1/egress_rpc";
import VideoPlayer from "../../../components/VideoPlayer";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { AsyncPassThrough } from "../../../lib/stream";
import VideoService from "../../mocks/video/service";

const Test: React.FC = () => {
  const [[service, client]] = React.useState((): [VideoService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new VideoService();
    registerEgressService(svc, service);

    const [a, b] = [new AsyncPassThrough(), new AsyncPassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => service.destroy(), [service]);

  return (
    <div className="video_mockup">
      <ApiProvider value={client}>
        <div className="video_mockup__content">
          <VideoPlayer
            networkKey="cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE"
            swarmUri="magnet:?xt=urn:ppspp:G43L6ZOLE6AODIYUJ57PALGMITCG26P46SINQRVQFSSLKWRDDR2Q&x.im=2&x.hf=4&x.sa=1&x.cs=1024&x.ps=32&x.sc=16"
            mimeType="video/mp4;codecs=mp4a.40.5,avc1.64001F"
            defaultControlsVisible={true}
            handleClose={() => undefined}
          />
        </div>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "BufferUnderrun",
    component: () => <Test />,
  },
];
