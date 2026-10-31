// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { MutableRefObject, useEffect, useState } from "react";

import { EgressOpenStreamResponse } from "../../apis/strims/video/v1/egress";
import { useClient } from "../../contexts/FrontendApi";
import { useSession } from "../../contexts/Session";
import useMediaRelay from "../../hooks/useMediaRelay";
import useMediaSource from "../../hooks/useMediaSource";
import useReady from "../../hooks/useReady";
import { VideoControls } from "../../hooks/useVideo";
import { LowLatencyPlayer } from "../../lib/media/lowlatency/player";
import { rendererParam } from "../../lib/media/lowlatency/support";
import { UnitsPlayer, attachHandle } from "../../lib/media/units";

export interface SwarmMediaProps {
  networkKey: string;
  swarmUri: string;
  mimeType: string;
  videoRef: MutableRefObject<HTMLVideoElement>;
  videoControls: VideoControls;
  /** A unit-format stream plays through the low-latency player, falling back to the baseline. */
  lowLatency?: boolean;
}

// Each component puts one way of playing a swarm on the video, rendering
// nothing. The player renders the one its stream's format takes, keyed by
// it, so a stream of another format mounts its own.

// A unit-format stream: decoded with WebCodecs by the low-latency player
// when asked for, until it gives up on the stream, else remuxed for
// MediaSource by the baseline player, each in a player worker. A node in
// the page feeds the baseline player's worker straight from the swarm;
// otherwise the stream comes over the Egress RPC, through the window.
export const UnitsMedia: React.FC<SwarmMediaProps> = ({
  networkKey,
  swarmUri,
  mimeType,
  videoRef,
  videoControls,
  lowLatency = false,
}) => {
  const client = useClient();
  const [{ conn }] = useSession();
  const feeds = conn?.feeds;
  const [fallback, setFallback] = useState(false);

  useEffect(() => {
    const video = videoRef.current;
    const request = {
      swarmUri,
      networkKeys: [Base64.toUint8Array(networkKey)],
      mimeType,
    };
    // eslint-disable-next-line no-console
    const ended = (message: string) => console.warn("the stream ended:", message);
    const baseline = !lowLatency || fallback;
    const player = !baseline
      ? new LowLatencyPlayer(video, {
          attach: (stream) => {
            videoControls.setSrc(null);
            video.srcObject = stream;
          },
          play: () => videoControls.play(),
          renderer: rendererParam(location.search) ?? undefined,
          error: (message, canFallBack) => {
            // eslint-disable-next-line no-console
            console.warn("the low-latency player can't play the stream:", message);
            if (canFallBack) setFallback(true);
          },
        })
      : new UnitsPlayer(video, {
          attach: (src) => {
            if (typeof src === "string") {
              video.srcObject = null;
              videoControls.setSrc(src);
            } else {
              videoControls.setSrc(null);
              attachHandle(video, src);
            }
          },
          play: () => videoControls.play(),
          // eslint-disable-next-line no-console
          error: (message) => console.warn("the stream can't play:", message),
          feed:
            feeds && ((port, replaces) => feeds.open(request, port, { replaces, closed: ended })),
        });

    if (baseline && feeds) {
      return () => {
        player.destroy();
        video.srcObject = null;
        videoControls.setSrc(null);
      };
    }

    // The stream's end, which only the baseline player plays out.
    const end = () => {
      if (player instanceof UnitsPlayer) player.end();
    };
    const stream = client.videoEgress.openStream(request);
    stream.on("data", ({ body }) => {
      switch (body.case) {
        case EgressOpenStreamResponse.BodyCase.DATA:
          player.push(body.data.data, body.data.segmentEnd, body.data.discontinuity);
          return;
        case EgressOpenStreamResponse.BodyCase.ERROR:
          ended(body.error.message);
          end();
          return;
      }
    });
    stream.on("end", end);
    stream.on("error", (e) => {
      // eslint-disable-next-line no-console
      console.warn("the stream failed:", e);
      end();
    });

    return () => {
      stream.destroy();
      player.destroy();
      video.srcObject = null;
      videoControls.setSrc(null);
    };
  }, [networkKey, swarmUri, mimeType, lowLatency, fallback, feeds]);

  return null;
};

// An fMP4 record stream, appended to a MediaSource in the window.
export const MediaSourceMedia: React.FC<SwarmMediaProps> = ({
  networkKey,
  swarmUri,
  mimeType,
  videoRef,
  videoControls,
}) => {
  const mediaSource = useMediaSource({ networkKey, swarmUri, mimeType, videoRef });

  useReady(() => {
    const src = URL.createObjectURL(mediaSource);
    videoControls.setSrc(src);
    return () => {
      URL.revokeObjectURL(src);
      videoControls.setSrc(null);
    };
  }, [mediaSource]);

  return null;
};

// An fMP4 record stream, relayed as HLS through the service worker.
export const MediaRelayMedia: React.FC<SwarmMediaProps> = ({
  networkKey,
  swarmUri,
  mimeType,
  videoRef,
  videoControls,
}) => {
  const src = useMediaRelay({ networkKey, swarmUri, mimeType, videoRef });

  useReady(() => {
    videoControls.setSrc(src);
    return () => videoControls.setSrc(null);
  }, [src]);

  return null;
};
