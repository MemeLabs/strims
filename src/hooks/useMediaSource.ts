import { MutableRefObject, useEffect, useMemo } from "react";
import { useGetSet } from "react-use";

import { useClient } from "../contexts/Api";
import * as fmp4 from "../lib/media/fmp4";
import * as mpegts from "../lib/media/mpegts";
import * as webm from "../lib/media/webm";

export type MimeType = "video/fmp4" | "video/mpegts" | "video/webm";

export interface MediaSourceProps {
  networkKey: Uint8Array;
  swarmKey: Uint8Array;
  mimeType: MimeType;
  videoRef: MutableRefObject<HTMLVideoElement>;
  // TODO: the rest of the swarm uri params
}

const useMediaSource = ({ networkKey, swarmKey, mimeType, videoRef }: MediaSourceProps) => {
  const client = useClient();

  return useMemo(() => {
    const Decoder = {
      "video/fmp4": fmp4.Decoder,
      "video/mpegts": mpegts.Decoder,
      "video/webm": webm.Decoder,
    }[mimeType];
    const decoder = new Decoder();

    const clientEvents = client.openVideoClient({
      swarmKey,
      emitData: true,
    });
    clientEvents.on("data", (e) => {
      switch (e.body) {
        case "open":
          // TODO: do this in the service
          client.publishSwarm({ id: e.open.id, networkKey });
          break;
        case "data":
          decoder.write(e.data.data);
          if (e.data.flush) {
            decoder.flush();

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

    return decoder.source.mediaSource;
  }, []);
};

export default useMediaSource;
