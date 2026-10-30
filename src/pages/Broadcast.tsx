// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useEffect, useRef, useState } from "react";

import { ImageType } from "../apis/strims/type/image";
import { useClient } from "../contexts/FrontendApi";
import {
  Capture,
  CaptureSender,
  MAX_FRAME_RATE,
  MAX_HEIGHT,
  MAX_WIDTH,
  captureBitrate,
} from "../lib/capture";
import { certificateRoot } from "../lib/certificate";

type Source = "screen" | "camera";

interface BroadcastState {
  id: Uint8Array;
  mediaStream: MediaStream;
  audioContext: AudioContext;
}

const SNIPPET = {
  title: "test broadcast",
  description: "broadcast from the browser",
};

const openMedia = (source: Source): Promise<MediaStream> =>
  source === "screen"
    ? navigator.mediaDevices.getDisplayMedia({
        video: {
          width: { max: MAX_WIDTH },
          height: { max: MAX_HEIGHT },
          frameRate: { max: MAX_FRAME_RATE },
        },
        audio: {
          autoGainControl: false,
          echoCancellation: false,
          noiseSuppression: false,
          suppressLocalAudioPlayback: false,
        },
      })
    : navigator.mediaDevices.getUserMedia({
        video: {
          width: { ideal: 1280 },
          height: { ideal: 720 },
          frameRate: { max: MAX_FRAME_RATE },
        },
        audio: true,
      });

const canShareScreen = (): boolean => !!navigator.mediaDevices?.getDisplayMedia;

const BroadcastPage: React.FC = () => {
  const client = useClient();
  const videoRef = useRef<HTMLVideoElement>();

  const [broadcastState, setBroadcastState] = useState<BroadcastState>(null);
  const [error, setError] = useState<string>(null);

  const handleStartBroadcastClick = (source: Source) => {
    setError(null);
    // Made in the click: an engine may run audio only in a context made in
    // a user gesture.
    const audioContext = new AudioContext({ sampleRate: 48000 });
    void (async () => {
      try {
        const mediaStream = await openMedia(source);
        const { networks } = await client.network.list();
        const { id } = await client.videoCapture.open({
          networkKeys: networks.map((n) => certificateRoot(n.certificate).key),
          directorySnippet: SNIPPET,
          bitrate: BigInt(captureBitrate(mediaStream)),
          video: mediaStream.getVideoTracks().length > 0,
        });
        setBroadcastState({ id, mediaStream, audioContext });
      } catch (e) {
        void audioContext.close();
        setError(String(e));
      }
    })();
  };

  const handleStopBroadcastClick = () => setBroadcastState(null);

  useEffect(() => {
    if (!videoRef.current || !broadcastState) {
      return;
    }

    const { id, mediaStream, audioContext } = broadcastState;
    videoRef.current.srcObject = mediaStream;
    videoRef.current.muted = true;

    const fail = (e: unknown) => setError(String(e));
    let capture: Capture | null = null;
    const sender = new CaptureSender(
      id,
      (req) => client.videoCapture.append(req),
      fail,
      () => capture?.requestKeyframe()
    );
    const started = (async (): Promise<Capture | null> => {
      try {
        capture = await Capture.start(mediaStream, sender, { audioContext, onerror: fail });
        return capture;
      } catch (e) {
        fail(e);
        return null;
      }
    })();
    // Sharing stopped from the browser's own controls.
    const [videoTrack] = mediaStream.getVideoTracks();
    videoTrack?.addEventListener("ended", () => setBroadcastState(null));

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
          ...SNIPPET,
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

    videoRef.current.addEventListener("playing", () => void generateThumbnail(), { once: true });
    const ivl = setInterval(() => void generateThumbnail(), 30 * 1000);

    return () => {
      clearInterval(ivl);
      void (async () => {
        const c = await started;
        await c?.stop();
        mediaStream.getTracks().forEach((t) => t.stop());
        await sender.drain();
        await client.videoCapture.close({ id });
        await audioContext.close();
      })().catch(() => undefined);
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
            <>
              {canShareScreen() && (
                <button
                  className="input input_button"
                  onClick={() => handleStartBroadcastClick("screen")}
                >
                  share screen
                </button>
              )}
              <button
                className="input input_button"
                onClick={() => handleStartBroadcastClick("camera")}
              >
                use camera
              </button>
            </>
          )}
          {error && <p>{error}</p>}
        </div>
        <video
          className="home_page__main__video__video"
          height="180"
          width="320"
          controls
          autoPlay
          playsInline
          ref={videoRef}
        />
      </section>
    </>
  );
};

export default BroadcastPage;
