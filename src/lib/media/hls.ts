// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import type { HLSEgressClient } from "../../apis/strims/video/v1/hls_egress_rpc";

/**
 * Milliseconds between the opens that keep an HLS stream from going idle
 * on the node while nothing fetches it: under a third of the node's 60 s
 * idle timeout, which an open of a stream it serves renews.
 */
export const RENEW_MS = 20_000;

/** The node calls HLS takes, of its frontend client. */
export type HlsApi = Pick<HLSEgressClient, "isSupported" | "getConfig" | "openStream">;

/** A stream as an HLS open names it: its listing's swarm URI, network and MIME type. */
export interface HlsRequest {
  swarmUri: string;
  networkKeys: Uint8Array[];
  mimeType: string;
}

/** Whether the node serves HLS: its driver does, and its HLS config is enabled. */
export const hlsServed = async (api: HlsApi): Promise<boolean> => {
  try {
    const [supported, config] = await Promise.all([api.isSupported(), api.getConfig()]);
    return supported.supported && !!config.config?.enabled;
  } catch {
    return false;
  }
};

/** The playlist URL of the stream's HLS stream on the node; null when it doesn't open. */
export const openHls = async (api: HlsApi, req: HlsRequest): Promise<string | null> => {
  try {
    const { playlistUrl } = await api.openStream(req);
    return playlistUrl || null;
  } catch {
    return null;
  }
};

/**
 * Whether a player on another device could fetch `url`: an http or https
 * URL whose host isn't a loopback address.
 */
export const reachable = (url: string): boolean => {
  let u: URL;
  try {
    u = new URL(url);
  } catch {
    return false;
  }
  if (u.protocol !== "http:" && u.protocol !== "https:") return false;
  const host = u.hostname.replace(/^\[|\]$/g, "").toLowerCase();
  return !(
    host === "localhost" ||
    host.endsWith(".localhost") ||
    host === "::1" ||
    /^127\.\d+\.\d+\.\d+$/.test(host)
  );
};

export interface LeaseTimers {
  setInterval: (f: () => void, ms: number) => unknown;
  clearInterval: (id: unknown) => void;
}

/**
 * Keeps the node's HLS stream of a stream open while nothing fetches it,
 * as an AirPlay source the video names but doesn't load: it opens the
 * stream again every RENEW_MS until closed.
 */
export class HlsLease {
  private readonly timer: unknown;

  constructor(
    private readonly api: HlsApi,
    private readonly req: HlsRequest,
    private readonly timers: LeaseTimers = {
      setInterval: (f, ms) => setInterval(f, ms),
      clearInterval: (id) => clearInterval(id as ReturnType<typeof setInterval>),
    }
  ) {
    this.timer = timers.setInterval(() => void openHls(this.api, this.req), RENEW_MS);
  }

  close(): void {
    this.timers.clearInterval(this.timer);
  }
}
