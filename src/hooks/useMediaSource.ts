// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { MutableRefObject, useEffect, useState } from "react";

import { EgressOpenStreamResponse } from "../apis/strims/video/v1/egress";
import { useClient } from "../contexts/FrontendApi";
import * as fmp4 from "../lib/media/fmp4";
import * as mpegts from "../lib/media/mpegts";
import * as webm from "../lib/media/webm";

const decoders = {
  "video/mp4": fmp4.Decoder,
  "video/mpeg-ts": mpegts.Decoder,
  "video/webm": webm.Decoder,
};

export type MimeType = keyof typeof decoders;

export interface MediaSourceProps {
  networkKey: string;
  swarmUri: string;
  mimeType: string;
  videoRef: MutableRefObject<HTMLVideoElement>;
  // TODO: the rest of the swarm uri params
}

const useMediaSource = ({
  networkKey,
  swarmUri,
  mimeType,
  videoRef,
}: MediaSourceProps): MediaSource => {
  const client = useClient();
  const [mediaSource, setMediaSource] = useState<MediaSource>(null);

  useEffect(() => {
    const [fileFormat] = mimeType.split(";", 1) as [MimeType];
    const Decoder = decoders[fileFormat];
    const decoder = new Decoder();
    let started = false;

    setMediaSource(decoder.source.getMediaSource());
    decoder.source.onReset = setMediaSource;

    console.log({
      swarmUri,
      mimeType,
      networkKeys: [networkKey],
    });

    const clientEvents = client.videoEgress.openStream({
      swarmUri,
      networkKeys: [Base64.toUint8Array(networkKey)],
    });
    clientEvents.on("data", ({ body }) => {
      switch (body.case) {
        case EgressOpenStreamResponse.BodyCase.DATA:
          if (body.data.bufferUnderrun) {
            decoder.reset();
            started = false;
          }

          decoder.write(body.data.data);

          if (body.data.segmentEnd) {
            decoder.flush();

            const [start, end] = decoder.source.bounds();
            if (!started && end - start >= 1) {
              started = true;
              videoRef.current.currentTime = end - 1;
              void videoRef.current.play();
            }

            decoder.source.prune(videoRef.current.currentTime - 10);
          }
      }
    });
    clientEvents.on("error", (e) => console.log(e));

    return () => clientEvents.destroy();
  }, [networkKey, swarmUri, mimeType, videoRef]);

  return mediaSource;
};

export default useMediaSource;
