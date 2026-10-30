// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { MutableRefObject, useEffect, useState } from "react";

import { EgressOpenStreamResponse } from "../apis/strims/video/v1/egress";
import { useClient } from "../contexts/FrontendApi";
import * as fmp4 from "../lib/media/fmp4";
import { StreamFormat, streamFormat } from "../lib/media/mime";

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
    // Only fMP4 records play through a window MediaSource; a stream of any
    // other format gets none.
    if (streamFormat(mimeType) !== StreamFormat.Fmp4) {
      // eslint-disable-next-line no-console
      console.warn("the stream can't play: no MediaSource path for", mimeType);
      setMediaSource(null);
      return;
    }

    const decoder = new fmp4.Decoder(mimeType);
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
      mimeType,
    });
    clientEvents.on("data", ({ body }) => {
      switch (body.case) {
        case EgressOpenStreamResponse.BodyCase.DATA:
          if (body.data.discontinuity) {
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
