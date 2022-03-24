import React, { useEffect, useRef, useState } from "react";

import { ImageType } from "../apis/strims/type/image";
import { useClient } from "../contexts/FrontendApi";
import { certificateRoot } from "../lib/certificate";
import * as webm from "../lib/media/webm";

const BroadcastPage: React.FC = () => {
  const client = useClient();
  const videoRef = useRef<HTMLVideoElement>();

  const [broadcastState, setBroadcastState] =
    useState<{ id: Uint8Array; mediaStream: MediaStream }>(null);

  const handleStartBroadcastClick = async () => {
    const { networks } = await client.network.list();
    const [{ id }, mediaStream] = await Promise.all([
      client.videoCapture.open({
        mimeType: webm.MIME_TYPE,
        networkKeys: networks.map((n) => certificateRoot(n.certificate).key),
        directorySnippet: {
          title: "test broadcast",
          description: "broadcast from getDisplayMedia",
        },
      }),

      navigator.mediaDevices.getDisplayMedia({
        video: {
          frameRate: 30,
        },
        audio: {
          autoGainControl: false,
          echoCancellation: false,
          noiseSuppression: false,
          suppressLocalAudioPlayback: false,

          // googEchoCancellation: false,
          // googAutoGainControl: false,
          // googNoiseSuppression: false,
          // googHighpassFilter: false,
        },
      }),
    ]);

    setBroadcastState({ id, mediaStream });
  };

  const handleStopBroadcastClick = () => setBroadcastState(null);

  useEffect(() => {
    if (!videoRef.current || !broadcastState) {
      return;
    }

    const { id, mediaStream } = broadcastState;
    const encoder = new webm.Encoder(mediaStream);

    videoRef.current.srcObject = mediaStream;
    videoRef.current.muted = true;

    encoder.ondata = (data) => void client.videoCapture.append({ id, data });
    encoder.onend = (data) => void client.videoCapture.append({ id, data, segmentEnd: true });

    const generateThumbnail = async () => {
      const { videoHeight, videoWidth } = videoRef.current;
      const maxHeight = 360;
      const maxWidth = 640;
      let height = videoHeight;
      let width = videoWidth;
      if (height > maxHeight) {
        width = (videoWidth * maxHeight) / videoHeight;
        height = maxHeight;
      }
      if (width > maxWidth) {
        height = (videoHeight * maxWidth) / videoWidth;
        width = maxWidth;
      }

      const canvas = document.createElement("canvas");
      canvas.height = height;
      canvas.width = width;
      const ctx = canvas.getContext("2d");
      ctx.drawImage(videoRef.current, 0, 0, videoWidth, videoHeight, 0, 0, width, height);

      const res = await fetch(canvas.toDataURL("image/jpeg", 3));
      const data = await res.arrayBuffer();

      await client.videoCapture.update({
        id,
        directorySnippet: {
          title: "test broadcast",
          description: "broadcast from getDisplayMedia",
          thumbnail: {
            sourceOneof: {
              image: {
                type: ImageType.IMAGE_TYPE_JPEG,
                data: new Uint8Array(data),
              },
            },
          },
        },
      });
    };

    videoRef.current.addEventListener("playing", generateThumbnail, { once: true });
    const ivl = setInterval(generateThumbnail, 30 * 1000);

    return () => {
      encoder.stop();
      void client.videoCapture.close({ id });
      mediaStream.getTracks().forEach((t) => t.stop());
      clearInterval(ivl);
    };
  }, [videoRef.current, broadcastState]);

  return (
    <>
      <header className="home_page__subheader"></header>
      <section className="home_page__main__video">
        <div>
          {broadcastState ? (
            <button className="input input_button" onClick={handleStopBroadcastClick}>
              stop broadcast
            </button>
          ) : (
            <button className="input input_button" onClick={handleStartBroadcastClick}>
              start broadcast
            </button>
          )}
        </div>
        <video
          className="home_page__main__video__video"
          height="180"
          width="320"
          controls
          autoPlay
          ref={videoRef}
        />
      </section>
    </>
  );
};

export default BroadcastPage;
