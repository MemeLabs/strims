/* eslint-disable no-console */

import * as React from "react";

import { MainLayout } from "../components/MainLayout";
import { useClient, useLazyCall } from "../contexts/Api";
import { useProfile } from "../contexts/Profile";
import { useTheme } from "../contexts/Theme";
import * as fmp4 from "../lib/media/fmp4";
import * as mpegts from "../lib/media/mpegts";
import * as webm from "../lib/media/webm";
import { CallChatClientRequest, OpenChatClientRequest, OpenChatServerRequest } from "../lib/pb";

const HomePage = () => {
  const [{ colorScheme }, { setColorScheme }] = useTheme();
  const [{ profile }, { clearProfile }] = useProfile();

  const client = useClient();

  const [pprofData, pprof] = useLazyCall("pprof");
  const [pprofDownloads, setPProfDownloads] = React.useState([]);

  const videoRef = React.useRef<HTMLVideoElement>();

  const addDownload = (name: string, data: Uint8Array) => {
    const f = new File([data], name, {
      type: "application/binary",
    });
    setPProfDownloads((prev) => [...prev, { name, url: URL.createObjectURL(f) }]);
  };

  React.useEffect(() => {
    if (!pprofData.value) {
      return;
    }
    addDownload(`${pprofData.value.name}.profile`, pprofData.value.data);
  }, [pprofData.value]);

  const handleStartVPNClick = () => client.startVPN();
  const handleStopVPNClick = async () => await client.stopVPN();

  const handleColorToggle = () => setColorScheme(colorScheme === "dark" ? "light" : "dark");
  const handleLogout = () => clearProfile();

  const handleStartBroadcastClick = async () => {
    const [{ id }, mediaStream] = await Promise.all([
      client.openVideoServer(),
      (navigator.mediaDevices as any).getDisplayMedia({
        video: true,
        audio: {
          autoGainControl: false,
          echoCancellation: false,
          googAutoGainControl: false,
          noiseSuppression: false,
        },
        frameRate: 30,
      }) as Promise<MediaStream>,
    ]);

    const encoder = new webm.Encoder(mediaStream);

    if (true) {
      broadcastEncoder(encoder, id, mediaStream);
    } else {
      debugEncoder(encoder);
    }
  };

  const debugEncoder = (encoder: webm.Encoder) => {
    const decoder = new webm.Decoder();

    encoder.ondata = (b) => {
      decoder.write(b);
      // console.log("write", b);
    };
    encoder.onend = (b) => {
      decoder.write(b);
      decoder.flush();
      // console.log("write/flush", b);
    };

    videoRef.current.src = URL.createObjectURL(decoder.source.mediaSource);
  };

  const broadcastEncoder = (encoder: webm.Encoder, id: number, mediaStream: MediaStream) => {
    videoRef.current.srcObject = mediaStream;

    encoder.ondata = (data) => client.writeToVideoServer({ id, data });
    encoder.onend = (data) => client.writeToVideoServer({ id, data, flush: true });

    publishSwarm(id);
  };

  const publishSwarm = async (id: number) => {
    const memeberships = await client.getNetworkMemberships();

    memeberships.networkMemberships.forEach((m) => {
      let rootCert = m.certificate;
      while (rootCert.parent) {
        rootCert = rootCert.parent;
      }

      client.publishSwarm({
        id,
        networkKey: rootCert.key,
      });
    });
  };

  const handleViewBroadcastClick = (decoder: webm.Decoder | fmp4.Decoder) => {
    const video = videoRef.current;
    video.src = URL.createObjectURL(decoder.source.mediaSource);
    video.oncanplay = () => video.play();

    const timeShifted = 0;

    const clientEvents = client.openVideoClient({ emitData: true });
    clientEvents.on("data", (v) => {
      switch (v.body) {
        case "open":
          publishSwarm(v.open.id);
          break;
        case "data":
          // console.log("read", v.data.data);

          decoder.write(v.data.data);
          if (v.data.flush) {
            decoder.flush();

            // if (timeShifted < 2) {
            //   timeShifted++;
            //   videoRef.current.currentTime = 999999999999;
            // }

            const end = decoder.source.end();
            const { currentTime } = videoRef.current;
            if (currentTime < end - 10) {
              videoRef.current.currentTime = end - 5;
              videoRef.current.play();

              console.log("skipping", videoRef.current.currentTime);
            }
          }
      }
    });
  };

  const handleTestClick = async () => {
    console.log("starting vpn");
    const networkEvents = client.startVPN();

    networkEvents.on("data", (e) => {
      console.log(e);
    });

    console.log("vpn started");

    console.log("waiting for networks...");
    await new Promise((resolve) => setTimeout(resolve, 2000));

    // console.log("starting swarm");
    // const swarm = await client.openVideoServer();
    // console.log("started swarm", swarm);

    // console.log("publishing swarm");
    // await client.publishSwarm({
    //   id: swarm.id,
    // });
    // console.log("swarm published");
  };

  const handleChatClientClick = async () => {
    const servers = await client.getChatServers();
    if (servers.chatServers.length === 0) {
      return;
    }

    const server = servers.chatServers[0];
    const chatEvents = client.openChatClient(
      new OpenChatClientRequest({
        networkKey: server.networkKey,
        serverKey: server.key.public,
      })
    );
    chatEvents.on("data", async (v) => {
      let n = 0;

      switch (v.body) {
        case "open":
          console.log("chat client >>>", v.open.clientId);

          await new Promise((resolve) => setTimeout(resolve, 2000));

          setInterval(() => {
            client.callChatClient(
              new CallChatClientRequest({
                clientId: v.open.clientId,
                message: new CallChatClientRequest.Message({
                  time: Date.now(),
                  body: `${n++}`,
                }),
              })
            );
          }, 500);
          return;
        case "close":
          console.log("chat client closed");
          return;
        case "message":
          console.log("chat message", v.message);
          console.log(
            "delay - e2e:",
            Date.now() - v.message.sentTime,
            "client>server:",
            v.message.serverTime - v.message.sentTime,
            "server>ui:",
            Date.now() - v.message.serverTime,
            "value:",
            v.message.body
          );
          return;
        default:
          console.log(v);
      }
    });
  };

  const handleChatServerClick = async () => {
    const servers = await client.getChatServers();
    if (servers.chatServers.length === 0) {
      return;
    }

    const server = servers.chatServers[0];
    const chatEvents = client.openChatServer(new OpenChatServerRequest({ server }));
    chatEvents.on("data", (v) => {
      switch (v.body) {
        case "open":
          console.log("chat server >>>", v.open.serverId);
          return;
        case "close":
          console.log("chat server closed");
          return;
        default:
          console.log(v);
      }
    });
  };

  const handleReadMetricsClick = async () => {
    const { data } = await client.readMetrics({ format: 0 });
    console.log(new TextDecoder().decode(data));
  };

  React.useEffect(() => {
    handleTestClick();
  }, []);

  return (
    <MainLayout>
      <main className="home_page__main">
        <header className="home_page__subheader"></header>
        <section className="home_page__main__video">
          <div>
            <button className="input input_button" onClick={handleColorToggle}>
              toggle theme
            </button>
            <button className="input input_button" onClick={handleStartVPNClick}>
              start vpn
            </button>
            <button className="input input_button" onClick={handleStopVPNClick}>
              stop vpn
            </button>
            <button className="input input_button" onClick={handleStartBroadcastClick}>
              start broadcast
            </button>
            <button
              className="input input_button"
              onClick={() => handleViewBroadcastClick(new webm.Decoder())}
            >
              view broadcast
            </button>
            <button
              className="input input_button"
              onClick={() => handleViewBroadcastClick(new fmp4.Decoder())}
            >
              view rtmp broadcast
            </button>
            <button className="input input_button" onClick={() => pprof({ name: "allocs" })}>
              allocs profile
            </button>
            <button className="input input_button" onClick={() => pprof({ name: "goroutine" })}>
              goroutine profile
            </button>
            <button className="input input_button" onClick={() => pprof({ name: "heap" })}>
              heap profile
            </button>
            <button className="input input_button" onClick={handleTestClick}>
              test
            </button>
            <button className="input input_button" onClick={handleChatClientClick}>
              chat client
            </button>
            <button className="input input_button" onClick={handleChatServerClick}>
              chat server
            </button>
            <button className="input input_button" onClick={handleReadMetricsClick}>
              read metrics
            </button>
          </div>
          <div>
            {pprofDownloads.map(({ name, url }, i) => (
              <a href={url} download={name} key={i}>
                {name}
              </a>
            ))}
          </div>
          <video className="home_page__main__video__video" controls autoPlay ref={videoRef} />
        </section>
      </main>
      <aside className="home_page__right">
        <header className="home_page__subheader"></header>
        <header className="home_page__chat__promo"></header>
        <div className="home_page__chat">chat</div>
      </aside>
    </MainLayout>
  );
};

export default HomePage;
