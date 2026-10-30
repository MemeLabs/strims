// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { Base64 } from "js-base64";
import React, { MutableRefObject, useEffect, useState } from "react";

import { EgressOpenStreamResponse } from "../../apis/strims/video/v1/egress";
import { useClient } from "../../contexts/FrontendApi";
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
// MediaSource by the baseline player, each in a player worker.
export const UnitsMedia: React.FC<SwarmMediaProps> = ({
  networkKey,
  swarmUri,
  mimeType,
  videoRef,
  videoControls,
  lowLatency = false,
}) => {
  const client = useClient();
  const [fallback, setFallback] = useState(false);

  useEffect(() => {
    const video = videoRef.current;
    const player =
      lowLatency && !fallback
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
          });

    const stream = client.videoEgress.openStream({
      swarmUri,
      networkKeys: [Base64.toUint8Array(networkKey)],
      mimeType,
    });
    stream.on("data", ({ body }) => {
      switch (body.case) {
        case EgressOpenStreamResponse.BodyCase.DATA:
          player.push(body.data.data, body.data.segmentEnd, body.data.discontinuity);
          return;
        case EgressOpenStreamResponse.BodyCase.ERROR:
          // eslint-disable-next-line no-console
          console.warn("the stream ended:", body.error.message);
          return;
      }
    });
    // eslint-disable-next-line no-console
    stream.on("error", (e) => console.warn("the stream failed:", e));

    return () => {
      stream.destroy();
      player.destroy();
      video.srcObject = null;
      videoControls.setSrc(null);
    };
  }, [networkKey, swarmUri, mimeType, lowLatency, fallback]);

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
