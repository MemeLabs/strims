// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import { reachable } from "../media/hls";

/** The Cast web sender SDK, with its application framework. */
export const CAST_SDK_URL =
  "https://www.gstatic.com/cv/js/sender/v1/cast_sender.js?loadCastFramework=1";
/** How long the SDK has to load and say whether the browser can cast. */
export const CAST_SDK_TIMEOUT_MS = 10_000;
/** The content type of an HLS stream, as the default media receiver takes it. */
export const HLS_CONTENT_TYPE = "application/x-mpegurl";

/** A media to load, as `chrome.cast.media.MediaInfo` holds it. */
export interface MediaInfo {
  contentId: string;
  contentType: string;
  contentUrl?: string;
  streamType?: string;
  hlsSegmentFormat?: string;
  hlsVideoSegmentFormat?: string;
  customData?: unknown;
}

/** A load, as `chrome.cast.media.LoadRequest` holds it. */
export interface LoadRequest {
  media: MediaInfo;
  autoplay?: boolean | null;
}

/** A Cast session, as `cast.framework.CastSession` gives it. */
export interface CastSession {
  getCastDevice(): { friendlyName: string };
  /** Rejects with a `chrome.cast.ErrorCode`. */
  loadMedia(request: LoadRequest): Promise<unknown>;
  getMediaSession(): { media?: { customData?: unknown } | null } | null;
}

export interface CastContextEvent {
  castState?: string;
  sessionState?: string;
}

/** The page's Cast context, as `cast.framework.CastContext` gives it. */
export interface CastContext {
  setOptions(options: { receiverApplicationId: string; autoJoinPolicy: string }): void;
  getCastState(): string;
  getCurrentSession(): CastSession | null;
  /** Shows the device picker; rejects with a `chrome.cast.ErrorCode`. */
  requestSession(): Promise<unknown>;
  endCurrentSession(stopCasting: boolean): void;
  addEventListener(type: string, handler: (e: CastContextEvent) => void): void;
}

/** The parts of the SDK the sender takes: `cast.framework` and `chrome.cast`. */
export interface CastSdk {
  framework: {
    CastContext: { getInstance(): CastContext };
    CastContextEventType: { CAST_STATE_CHANGED: string; SESSION_STATE_CHANGED: string };
    CastState: { NO_DEVICES_AVAILABLE: string; CONNECTING: string };
    SessionState: { SESSION_STARTED: string; SESSION_RESUMED: string; SESSION_ENDED: string };
  };
  cast: {
    AutoJoinPolicy: { ORIGIN_SCOPED: string };
    media: {
      DEFAULT_MEDIA_RECEIVER_APP_ID: string;
      MediaInfo: new (contentId: string, contentType: string) => MediaInfo;
      LoadRequest: new (media: MediaInfo) => LoadRequest;
      StreamType: { LIVE: string };
      HlsSegmentFormat: { FMP4: string };
      HlsVideoSegmentFormat: { FMP4: string };
    };
  };
}

interface CastWindow {
  isSecureContext: boolean;
  navigator: { userAgent: string };
  document: Document;
  chrome?: { cast?: CastSdk["cast"] };
  cast?: { framework?: CastSdk["framework"] };
  __onGCastApiAvailable?: (available: boolean) => void;
  setTimeout: (f: () => void, ms: number) => unknown;
}

/**
 * Whether the browser may have Chrome's Cast sender: Chromium's `chrome`
 * object, on a secure page, outside iOS, whose browsers are WebKit's.
 * The SDK says for sure once loaded.
 */
export const mayCast = (win: CastWindow): boolean =>
  win.isSecureContext && "chrome" in win && !/iPhone|iPad|iPod/.test(win.navigator.userAgent);

const loads = new WeakMap<object, Promise<CastSdk | null>>();

/**
 * Loads the Cast SDK into the page, once: its API when the browser can
 * cast, and null when it can't, the script fails, or it says nothing for
 * CAST_SDK_TIMEOUT_MS. Where the browser can't have the sender, nothing
 * loads.
 */
export const loadCastSdk = (win: CastWindow = window): Promise<CastSdk | null> => {
  let load = loads.get(win);
  if (!load) {
    load = mayCast(win) ? inject(win) : Promise.resolve<CastSdk | null>(null);
    loads.set(win, load);
  }
  return load;
};

const inject = (win: CastWindow): Promise<CastSdk | null> =>
  new Promise((resolve) => {
    win.__onGCastApiAvailable = (available) => {
      const framework = win.cast?.framework;
      const cast = win.chrome?.cast;
      resolve(available && framework && cast ? { framework, cast } : null);
    };
    const script = win.document.createElement("script");
    script.src = CAST_SDK_URL;
    script.async = true;
    script.onerror = () => resolve(null);
    win.document.head.appendChild(script);
    win.setTimeout(() => resolve(null), CAST_SDK_TIMEOUT_MS);
  });

/** What the page's Cast sender is doing. */
export interface CastState {
  /** A receiver is on the network, or a session lasts. */
  available: boolean;
  /** A session is starting. */
  connecting: boolean;
  /** The receiver's name, while a session lasts. */
  device: string | null;
  /** The swarm URI of the stream the receiver plays, of those this page's senders loaded. */
  casting: string | null;
  /** Why the last cast failed, until the next. */
  error: string | null;
}

/** A stream to cast: its swarm URI, and its HLS playlist URL, which may come later. */
export interface CastStream {
  swarmUri: string;
  url: Promise<string | null>;
}

/** Whether a player shows its Cast button: the SDK loaded, the node serves HLS and a receiver is there. */
export const castButtonVisible = (sdk: boolean, served: boolean, state: CastState | null) =>
  sdk && served && !!state?.available;

/** The swarm URI a media's custom data names. */
const swarmUriOf = (data: unknown): string | null => {
  const uri = (data as { swarmUri?: unknown } | null | undefined)?.swarmUri;
  return typeof uri === "string" ? uri : null;
};

/**
 * Casts streams to Cast's default media receiver, which plays their HLS
 * playlists: live fMP4 HLS, at the node's HLS URL. One per page, over the
 * page's Cast context; sessions the context resumes after a reload come
 * with the stream they play.
 */
export class CastSender {
  private readonly context: CastContext;
  private current: CastState;
  private readonly listeners = new Set<(state: CastState) => void>();

  constructor(private readonly sdk: CastSdk) {
    this.context = sdk.framework.CastContext.getInstance();
    this.context.setOptions({
      receiverApplicationId: sdk.cast.media.DEFAULT_MEDIA_RECEIVER_APP_ID,
      autoJoinPolicy: sdk.cast.AutoJoinPolicy.ORIGIN_SCOPED,
    });
    this.current = {
      available: false,
      connecting: false,
      device: null,
      casting: null,
      error: null,
    };
    this.castStateChanged(this.context.getCastState());
    this.sessionChanged();
    const events = sdk.framework.CastContextEventType;
    this.context.addEventListener(events.CAST_STATE_CHANGED, (e) =>
      this.castStateChanged(e.castState)
    );
    this.context.addEventListener(events.SESSION_STATE_CHANGED, (e) => {
      if (e.sessionState === sdk.framework.SessionState.SESSION_ENDED) {
        this.set({ device: null, casting: null });
      } else {
        this.sessionChanged();
      }
    });
  }

  state(): CastState {
    return this.current;
  }

  /** Calls `listener` with each new state, until the returned function is called. */
  subscribe(listener: (state: CastState) => void): () => void {
    this.listeners.add(listener);
    return () => this.listeners.delete(listener);
  }

  /**
   * Casts `stream`: in the session there is, or one the device picker
   * starts. Call it from a user gesture, which the picker needs; the
   * stream's URL may resolve after. A session it started ends when the
   * stream has no URL a receiver can reach.
   */
  async cast(stream: CastStream): Promise<void> {
    let session = this.context.getCurrentSession();
    const started = !session;
    if (!session) {
      this.set({ connecting: true, error: null });
      try {
        await this.context.requestSession();
      } catch (e) {
        // The viewer closing the picker is no failure.
        this.set({ connecting: false, error: e === "cancel" ? null : `casting failed: ${e}` });
        return;
      }
      session = this.context.getCurrentSession();
      this.set({ connecting: false });
      if (!session) return;
    }
    const url = await stream.url;
    const error = !url
      ? "the node doesn't serve this stream as HLS"
      : !reachable(url)
      ? `a receiver can't reach the node's HLS address (${
          new URL(url).host
        }): set its public address`
      : null;
    if (error) {
      this.set({ error });
      if (started) this.context.endCurrentSession(true);
      return;
    }
    const m = this.sdk.cast.media;
    const media = new m.MediaInfo(url, HLS_CONTENT_TYPE);
    media.contentUrl = url;
    media.streamType = m.StreamType.LIVE;
    media.hlsSegmentFormat = m.HlsSegmentFormat.FMP4;
    media.hlsVideoSegmentFormat = m.HlsVideoSegmentFormat.FMP4;
    media.customData = { swarmUri: stream.swarmUri };
    const request = new m.LoadRequest(media);
    request.autoplay = true;
    try {
      await session.loadMedia(request);
      this.set({ casting: stream.swarmUri, error: null });
    } catch (e) {
      this.set({ error: `the receiver can't play the stream: ${e}` });
    }
  }

  /** Stops casting: ends the session, and the receiver's playback. */
  stop(): void {
    this.context.endCurrentSession(true);
  }

  private castStateChanged(state: string | undefined): void {
    const s = this.sdk.framework.CastState;
    this.set({
      available: state !== undefined && state !== s.NO_DEVICES_AVAILABLE,
      connecting: state === s.CONNECTING,
    });
  }

  private sessionChanged(): void {
    const session = this.context.getCurrentSession();
    if (!session) return;
    const media = session.getMediaSession()?.media;
    this.set({
      device: session.getCastDevice().friendlyName,
      casting: media ? swarmUriOf(media.customData) : this.current.casting,
    });
  }

  private set(change: Partial<CastState>): void {
    this.current = { ...this.current, ...change };
    for (const l of this.listeners) l(this.current);
  }
}

let sender: Promise<CastSender | null> | null = null;

/** The page's Cast sender, once the SDK loads; null where the browser can't cast. */
export const castSender = (): Promise<CastSender | null> => {
  sender ??= loadCastSdk().then((sdk) => (sdk ? new CastSender(sdk) : null));
  return sender;
};
