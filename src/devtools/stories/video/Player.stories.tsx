import { PassThrough } from "stream";

import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React from "react";

import { FrontendClient } from "../../../apis/client";
import { registerEgressService } from "../../../apis/strims/video/v1/egress_rpc";
import { MainLayoutProvider } from "../../../components/MainLayout";
import VideoPlayer from "../../../components/VideoPlayer";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import VideoService from "../../mocks/video/service";

const Test: React.FC = () => {
  const [[service, client]] = React.useState((): [VideoService, FrontendClient] => {
    const svc = new ServiceRegistry();
    const service = new VideoService();
    registerEgressService(svc, service);

    const [a, b] = [new PassThrough(), new PassThrough()];
    new Host(a, b, svc);
    return [service, new FrontendClient(b, a)];
  });

  React.useEffect(() => () => service.destroy(), [service]);

  return (
    <div className="video_mockup app app--dark">
      <ApiProvider value={client}>
        <MainLayoutProvider>
          <div className="video_mockup__content">
            <VideoPlayer
              networkKey="cgqhekoCTcy7OOkRdbNbYG3J4svZorYlH3KKaT660BE"
              swarmUri="magnet:?xt=urn:ppspp:G43L6ZOLE6AODIYUJ57PALGMITCG26P46SINQRVQFSSLKWRDDR2Q&x.im=2&x.hf=4&x.sa=1&x.cs=1024&x.ps=32&x.sc=16"
              mimeType="video/mp4;codecs=mp4a.40.5,avc1.64001F"
              defaultControlsVisible={true}
            />
          </div>
        </MainLayoutProvider>
      </ApiProvider>
    </div>
  );
};

export default [
  {
    name: "buffer underrun",
    component: () => <Test />,
  },
];
