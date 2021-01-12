import { MutableRefObject, useEffect, useMemo } from "react";
import { useGetSet } from "react-use";

import { useClient } from "../contexts/Api";
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
  networkKey: Uint8Array;
  swarmKey: Uint8Array;
  mimeType: MimeType;
  videoRef: MutableRefObject<HTMLVideoElement>;
  // TODO: the rest of the swarm uri params
}

const useMediaSource = ({ networkKey, swarmKey, mimeType, videoRef }: MediaSourceProps) => {
  const client = useClient();

  return useMemo(() => {
    const Decoder = decoders[mimeType];
    const decoder = new Decoder();
    // let started = false;

    // const clientEvents = client.video.openClient({
    //   swarmKey,
    //   emitData: true,
    // });
    // clientEvents.on("data", (e) => {
    //   switch (e.body) {
    //     case "open":
    //       // TODO: do this in the service
    //       client.video.publishSwarm({ id: e.open.id, networkKey });
    //       break;
    //     case "data":
    //       decoder.write(e.data.data);
    //       if (e.data.flush) {
    //         decoder.flush();

    //         const [start, end] = decoder.source.bounds();
    //         if (!started && end - start >= 1) {
    //           started = true;
    //           videoRef.current.currentTime = end - 1;
    //           videoRef.current.play();
    //         }
    //       }
    //   }
    // });

    return decoder.source.mediaSource;
  }, []);
};

export default useMediaSource;
