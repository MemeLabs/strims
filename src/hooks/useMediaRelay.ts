// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import { MutableRefObject, useEffect, useState } from "react";

import { EgressOpenStreamResponse } from "../apis/strims/video/v1/egress";
import { useClient } from "../contexts/FrontendApi";
import * as fmp4 from "../lib/media/fmp4";
import * as mpegts from "../lib/media/mpegts";

const relays = {
  "video/mp4": fmp4.Relay,
  "video/mpeg-ts": mpegts.Relay,
};

export type MimeType = keyof typeof relays;

export interface MediaRelayProps {
  networkKey: string;
  swarmUri: string;
  mimeType: string;
  videoRef: MutableRefObject<HTMLVideoElement>;
}

// TODO: merge with useMediaSource
const useMediaRelay = ({ networkKey, swarmUri, mimeType, videoRef }: MediaRelayProps): string => {
  const client = useClient();
  const [url, setUrl] = useState<string>(null);

  useEffect(() => {
    const [fileFormat] = mimeType.split(";", 1) as [MimeType];
    const Relay = relays[fileFormat];
    const relay = new Relay();

    relay.playlist.onReset = setUrl;

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
          if (body.data.discontinuity) {
            relay.reset();
          }

          relay.write(body.data.data);

          if (body.data.segmentEnd) {
            relay.flush();
          }
      }
    });
    clientEvents.on("error", (e) => console.log(e));

    return () => {
      clientEvents.destroy();
      relay.playlist.close();
    };
  }, [networkKey, swarmUri, mimeType, videoRef]);

  return url;
};

export default useMediaRelay;
