// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import "./RemotePlayback.scss";

import { Base64 } from "js-base64";
import React, { MutableRefObject, ReactNode, useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { IconType } from "react-icons/lib";
import { MdAirplay, MdCast, MdCastConnected } from "react-icons/md";

import { useClient } from "../../contexts/FrontendApi";
import { CastSender, CastState, castButtonVisible, castSender } from "../../lib/cast/sender";
import { airPlayCapable, showAirPlayPicker, watchAirPlay } from "../../lib/media/airplay";
import { HlsApi, HlsLease, HlsRequest, hlsServed, openHls, reachable } from "../../lib/media/hls";

/** How long a cast's failure shows. */
const ERROR_MS = 6000;

interface RemotePlaybackProps {
  networkKey: string;
  swarmUri: string;
  mimeType: string;
  videoRef: MutableRefObject<HTMLVideoElement>;
}

export interface RemotePlayback {
  /** The stream's HLS URL as the video's AirPlay source, once a receiver is available. */
  airPlaySource: string | null;
  /** A Cast receiver plays this stream, so the video here plays nothing. */
  casting: boolean;
  /** The Cast and AirPlay buttons, for the player's controls. */
  buttons: ReactNode;
  /** What shows over the video while it casts, or after a cast failed. */
  overlay: ReactNode;
}

interface ButtonProps {
  className: string;
  tooltip: string;
  icon: IconType;
  onClick: () => void;
}

// The player controls' button markup.
const Button: React.FC<ButtonProps> = ({ className, tooltip, icon: Icon, onClick }) => (
  <div className={`button-wrap ${className}`}>
    <button data-tip={tooltip} onClick={onClick}>
      <Icon />
    </button>
  </div>
);

// The page's Cast sender and its state, where the browser can cast, and
// whether the node serves HLS, asked once the sender is there.
const useCast = (api: HlsApi, request: HlsRequest) => {
  const [sender, setSender] = useState<CastSender | null>(null);
  const [state, setState] = useState<CastState | null>(null);
  const [served, setServed] = useState(false);

  useEffect(() => {
    let live = true;
    let unsubscribe: (() => void) | null = null;
    void castSender().then((s) => {
      if (!live || !s) return;
      setSender(s);
      setState(s.state());
      unsubscribe = s.subscribe(setState);
    });
    return () => {
      live = false;
      unsubscribe?.();
    };
  }, []);

  useEffect(() => {
    if (!sender) return;
    let live = true;
    void hlsServed(api).then((ok) => live && setServed(ok));
    return () => {
      live = false;
    };
  }, [sender, request]);

  return { sender, state, served };
};

// Whether an AirPlay receiver is available, where the browser can AirPlay,
// and once one is, the stream's HLS URL as its AirPlay source, kept open
// on the node while the player shows. Viewers without a receiver ask the
// node nothing.
const useAirPlay = (api: HlsApi, request: HlsRequest) => {
  const [available, setAvailable] = useState(false);
  const [source, setSource] = useState<{ request: HlsRequest; url: string } | null>(null);
  const url = source?.request === request ? source.url : null;

  useEffect(() => (airPlayCapable() ? watchAirPlay(setAvailable) : undefined), []);

  useEffect(() => {
    if (!available || url) return;
    let live = true;
    void (async () => {
      if (!(await hlsServed(api))) return;
      const opened = await openHls(api, request);
      if (live && opened && reachable(opened)) setSource({ request, url: opened });
    })();
    return () => {
      live = false;
    };
  }, [available, url, request]);

  useEffect(() => {
    if (!url) return;
    const lease = new HlsLease(api, request);
    return () => lease.close();
  }, [url, request]);

  return { available, url };
};

/**
 * Plays the stream elsewhere: on a Cast receiver from Chrome's Cast
 * sender, or on an AirPlay receiver through Safari, both from the HLS
 * stream the node serves of it. Each shows only where the browser can,
 * the node serves HLS, and a receiver is there.
 */
export const useRemotePlayback = ({
  networkKey,
  swarmUri,
  mimeType,
  videoRef,
}: RemotePlaybackProps): RemotePlayback => {
  const { t } = useTranslation();
  const client = useClient();
  const api = client.hlsEgress;
  const request = useMemo(
    () => ({ swarmUri, networkKeys: [Base64.toUint8Array(networkKey)], mimeType }),
    [networkKey, swarmUri, mimeType]
  );
  const cast = useCast(api, request);
  const airPlay = useAirPlay(api, request);

  const error = cast.state?.error ?? null;
  const [shownError, setShownError] = useState<string | null>(null);
  useEffect(() => {
    setShownError(error);
    if (!error) return;
    const timer = setTimeout(() => setShownError(null), ERROR_MS);
    return () => clearTimeout(timer);
  }, [error]);

  const casting = cast.state?.casting === swarmUri;
  const toggleCast = () => {
    if (casting) {
      cast.sender.stop();
    } else {
      void cast.sender.cast({ swarmUri, url: openHls(api, request) });
    }
  };

  const buttons = (
    <>
      {castButtonVisible(!!cast.sender, cast.served, cast.state) && (
        <Button
          className="cast"
          tooltip={casting ? t("player.Stop casting") : t("player.Cast")}
          icon={cast.state.device ? MdCastConnected : MdCast}
          onClick={toggleCast}
        />
      )}
      {airPlay.available && airPlay.url && (
        <Button
          className="airplay"
          tooltip={t("player.AirPlay")}
          icon={MdAirplay}
          onClick={() => showAirPlayPicker(videoRef.current)}
        />
      )}
    </>
  );

  const overlay = (
    <>
      {casting && (
        <div className="video_player__casting">
          <MdCastConnected />
          <span>{t("player.Casting to", { device: cast.state.device ?? "" })}</span>
          <button onClick={() => cast.sender.stop()}>{t("player.Stop casting")}</button>
        </div>
      )}
      {shownError && <div className="video_player__cast_error">{shownError}</div>}
    </>
  );

  return { airPlaySource: airPlay.url, casting, buttons, overlay };
};
