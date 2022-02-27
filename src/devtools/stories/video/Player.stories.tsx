import Host from "@memelabs/protobuf/lib/rpc/host";
import ServiceRegistry from "@memelabs/protobuf/lib/rpc/service";
import React, { useEffect, useState } from "react";

import { FrontendClient } from "../../../apis/client";
import { registerEgressService } from "../../../apis/strims/video/v1/egress_rpc";
import VideoPlayer from "../../../components/VideoPlayer";
import { Provider as ApiProvider } from "../../../contexts/FrontendApi";
import { Provider as PlayerProvider } from "../../../contexts/Player";
import { AsyncPassThrough } from "../../../lib/stream";
import vid0 from "../../mocks/video/sample/0.m4s";
import vid1 from "../../mocks/video/sample/1.m4s";
import vid2 from "../../mocks/video/sample/2.m4s";
import vid3 from "../../mocks/video/sample/3.m4s";
import vid4 from "../../mocks/video/sample/4.m4s";
import vid5 from "../../mocks/video/sample/5.m4s";
import vid6 from "../../mocks/video/sample/6.m4s";
import vid7 from "../../mocks/video/sample/7.m4s";
import vid8 from "../../mocks/video/sample/8.m4s";
import vid9 from "../../mocks/video/sample/9.m4s";
import vidInit from "../../mocks/video/sample/init.mp4";
import vidPlaylist from "../../mocks/video/sample/playlist.m3u8";
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
          />
        </div>
      </ApiProvider>
    </div>
  );
};

const NativeHLS: React.FC = () => {
  const [playlistUrl, setPlaylistUrl] = useState("");

  useEffect(() => {
    const cacheName = "hls-test";

    void (async () => {
      const cache = await caches.open(cacheName);
      await Promise.all(
        Object.entries({
          "0.m4s": {
            url: vid0,
            type: "video/iso.segment",
          },
          "1.m4s": {
            url: vid1,
            type: "video/iso.segment",
          },
          "2.m4s": {
            url: vid2,
            type: "video/iso.segment",
          },
          "3.m4s": {
            url: vid3,
            type: "video/iso.segment",
          },
          "4.m4s": {
            url: vid4,
            type: "video/iso.segment",
          },
          "5.m4s": {
            url: vid5,
            type: "video/iso.segment",
          },
          "6.m4s": {
            url: vid6,
            type: "video/iso.segment",
          },
          "7.m4s": {
            url: vid7,
            type: "video/iso.segment",
          },
          "8.m4s": {
            url: vid8,
            type: "video/iso.segment",
          },
          "9.m4s": {
            url: vid9,
            type: "video/iso.segment",
          },
          "init.mp4": {
            url: vidInit,
            type: "video/mp4",
          },
          "playlist.m3u8": {
            url: vidPlaylist,
            type: "application/vnd.apple.mpegurl",
          },
        }).map(async ([name, { url, type }]) => {
          const res = await fetch(url);
          const b = await res.arrayBuffer().then((b) => new Uint8Array(b));

          await cache.put(
            new Request(`${location.origin}/_cache/${cacheName}/${name}`),
            new Response(new Blob([b], { type }))
          );
        })
      );
      setPlaylistUrl(`${location.origin}/_cache/${cacheName}/playlist.m3u8`);
    })();

    return () => caches.delete(cacheName);
  }, []);

  if (!playlistUrl) {
    return null;
  }

  return <video src={playlistUrl} controls autoPlay height="360" width="640" />;
};

export default [
  {
    name: "BufferUnderrun",
    component: () => <Test />,
  },
  {
    name: "NativeHLS",
    component: () => <NativeHLS />,
  },
];
