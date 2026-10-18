// Copyright 2026 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

// The window's side of a strims-wasm node. The node runs in a dedicated
// worker; the window hands it a port for the frontend RPC connection and a
// port for signalling, hosts its RTCPeerConnections and forwards visibility
// changes. The node's PSI sessions compute in a second worker, over a port
// between the two.

import { EventEmitter } from "events";

import { IS_IOS } from "../lib/userAgent";

export interface WasmNodeOptions {
  /** A zap level: -1 debug, 0 info, 1 warn, 2 error. */
  logLevel: number;
  /** Allows dialing ws:// URIs. */
  allowInsecureWs: boolean;
  /** ICE server URLs, e.g. "stun:stun.example.com:3478". */
  iceServers: string[];
  /** Starts the bare node the browser link tests control. */
  e2e?: boolean;
}

/** The PSI worker's options. */
export type PsiWorkerOptions = Pick<WasmNodeOptions, "logLevel">;

/**
 * The one message the window sends each worker, with its ports transferred.
 * The node's worker gets `psi` when a PSI worker serves the other end. That
 * port carries the PSI sessions' keys and seeds, so it connects this page's
 * two workers and nothing else.
 */
export type InitMessage =
  | {
      type: "init";
      role: "node";
      options: WasmNodeOptions;
      frontend: MessagePort;
      signal: MessagePort;
      psi?: MessagePort;
    }
  | { type: "init"; role: "psi"; options: PsiWorkerOptions; port: MessagePort };

/** What the PSI worker posts the window when it can't start. */
export type PsiWorkerMessage = { type: "fatal"; message: string };

/** Messages the worker sends on the signal port. Conn ids are JS-safe integers. */
export type WorkerSignal =
  | { type: "create"; conn: number; iceServers: string[]; gatherTimeoutMs: number }
  | { type: "describe"; conn: number; sdpType: "offer" | "answer" }
  | { type: "remote"; conn: number; description: string }
  | { type: "close"; conn: number }
  | { type: "fatal"; message: string };

/** Messages the window sends on the signal port. */
export type WindowSignal =
  | { type: "channel"; conn: number; channel: RTCDataChannel }
  | { type: "local"; conn: number; description: string }
  | { type: "connected"; conn: number; maxMessageSize: number }
  | { type: "closed"; conn: number; reason: string }
  | { type: "visibility"; hidden: boolean; forceHUP: boolean };

/** Every live peer connection of every node, for the debugger. */
const peerConnections: RTCPeerConnection[] = [];
(window as unknown as DebugWindow).__strims_rtc_peer_connections__ = peerConnections;

/**
 * The frontend RPC connection. Each message on the port is one ArrayBuffer
 * of whole varint-delimited Calls. It emits `data` with a Uint8Array per
 * message, and `close` once the node is gone.
 */
export class PortBus extends EventEmitter {
  private closed = false;

  constructor(private port: MessagePort) {
    super();
    port.onmessage = ({ data }: MessageEvent<ArrayBuffer>) => {
      this.emit("data", new Uint8Array(data));
    };
  }

  // Sends one message. A view of a whole buffer is transferred, so its
  // buffer is the bus's afterwards; any other view is copied.
  write(data: Uint8Array): void {
    const { buffer } = data;
    if (
      buffer instanceof ArrayBuffer &&
      data.byteOffset === 0 &&
      data.byteLength === buffer.byteLength
    ) {
      this.port.postMessage(buffer, [buffer]);
    } else {
      const copy = data.slice().buffer;
      this.port.postMessage(copy, [copy]);
    }
  }

  // Closes the port and emits `close`, the first time it is called.
  shut(): void {
    if (this.closed) return;
    this.closed = true;
    this.port.onmessage = null;
    this.port.close();
    this.emit("close");
  }
}

// Resolves once ICE gathering completes, or after timeoutMs.
const gathered = (pc: RTCPeerConnection, timeoutMs: number) =>
  new Promise<void>((resolve) => {
    if (pc.iceGatheringState === "complete") {
      resolve();
      return;
    }
    const done = () => {
      clearTimeout(timer);
      pc.removeEventListener("icegatheringstatechange", onChange);
      resolve();
    };
    const onChange = () => {
      if (pc.iceGatheringState === "complete") done();
    };
    const timer = setTimeout(done, timeoutMs);
    pc.addEventListener("icegatheringstatechange", onChange);
  });

interface Peer {
  pc: RTCPeerConnection;
  gatherTimeoutMs: number;
  /** The connection's operations, which run in the order they arrive. */
  chain: Promise<void>;
  connected: boolean;
}

/** Hosts one node's peer connections and answers its signalling. */
class PeerHost {
  private peers = new Map<number, Peer>();

  constructor(private post: (msg: WindowSignal, transfer?: Transferable[]) => void) {}

  handle(msg: WorkerSignal): void {
    switch (msg.type) {
      case "create":
        this.create(msg.conn, msg.iceServers, msg.gatherTimeoutMs);
        break;
      case "describe":
        this.enqueue(msg.conn, async ({ pc, gatherTimeoutMs }) => {
          const description =
            msg.sdpType === "offer" ? await pc.createOffer() : await pc.createAnswer();
          await pc.setLocalDescription(description);
          await gathered(pc, gatherTimeoutMs);
          return {
            type: "local",
            conn: msg.conn,
            description: JSON.stringify(pc.localDescription),
          };
        });
        break;
      case "remote":
        this.enqueue(msg.conn, async ({ pc }) => {
          await pc.setRemoteDescription(JSON.parse(msg.description) as RTCSessionDescriptionInit);
        });
        break;
      case "close":
        this.enqueue(msg.conn, (peer) => {
          this.forget(msg.conn, peer);
          return Promise.resolve();
        });
        break;
    }
  }

  closeAll(): void {
    for (const [conn, peer] of this.peers) this.forget(conn, peer);
  }

  // Creates the connection and its negotiated channel, and transfers the
  // channel to the worker. A channel can be transferred only before it is
  // used and before the task that created it ends, so nothing here may await.
  private create(conn: number, iceServers: string[], gatherTimeoutMs: number) {
    const old = this.peers.get(conn);
    if (old) this.forget(conn, old);

    let pc: RTCPeerConnection;
    try {
      pc = new RTCPeerConnection({ iceServers: iceServers.map((urls) => ({ urls })) });
    } catch (e) {
      this.post({ type: "closed", conn, reason: String(e) });
      return;
    }
    const peer: Peer = { pc, gatherTimeoutMs, chain: Promise.resolve(), connected: false };
    this.peers.set(conn, peer);
    peerConnections.push(pc);
    pc.onconnectionstatechange = () => this.onConnectionStateChange(conn, peer);

    try {
      const channel = pc.createDataChannel("data", { negotiated: true, id: 1, ordered: true });
      this.post({ type: "channel", conn, channel }, [channel]);
    } catch (e) {
      this.fail(conn, peer, e);
    }
  }

  private onConnectionStateChange(conn: number, peer: Peer) {
    if (this.peers.get(conn) !== peer) return;
    const { pc } = peer;
    switch (pc.connectionState) {
      case "connected":
        if (!peer.connected) {
          peer.connected = true;
          this.post({ type: "connected", conn, maxMessageSize: pc.sctp?.maxMessageSize ?? 0 });
        }
        break;
      case "failed":
      case "closed":
        this.fail(conn, peer, `connection ${pc.connectionState}`);
        break;
    }
  }

  // Runs op after the connection's earlier operations, and posts what it
  // returns. A rejected operation closes the connection.
  private enqueue(conn: number, op: (peer: Peer) => Promise<WindowSignal | void>) {
    const peer = this.peers.get(conn);
    if (!peer) return;
    peer.chain = peer.chain
      .then(async () => {
        if (this.peers.get(conn) !== peer) return;
        const reply = await op(peer);
        if (reply && this.peers.get(conn) === peer) this.post(reply);
      })
      .catch((e) => this.fail(conn, peer, e));
  }

  // Closes the connection and tells the worker why, if it is still live.
  private fail(conn: number, peer: Peer, reason: unknown) {
    if (this.peers.get(conn) !== peer) return;
    this.forget(conn, peer);
    this.post({ type: "closed", conn, reason: String(reason) });
  }

  private forget(conn: number, peer: Peer) {
    if (this.peers.get(conn) === peer) this.peers.delete(conn);
    peer.pc.onconnectionstatechange = null;
    peer.pc.close();
    const i = peerConnections.indexOf(peer.pc);
    if (i !== -1) peerConnections.splice(i, 1);
  }
}

export interface WasmNode {
  bus: PortBus;
  /** Terminates the workers and closes the node's peer connections. */
  close(): void;
}

// Both workers run svc.worker.ts. It is constructed in one place, so webpack
// bundles it once.
const spawnWorker = (name: "svc" | "psi") =>
  new Worker(/* webpackChunkName: "svc" */ new URL("./svc.worker.ts", import.meta.url), {
    type: "module",
    name,
  });

// Starts the worker that computes a node's PSI sessions, serving `port`.
// Without it the node's PSI sessions stall and its network control gives up
// on them, so a PSI worker that dies is logged and the node runs on.
const startPsiWorker = (logLevel: number, port: MessagePort): Worker => {
  const worker = spawnWorker("psi");
  const died = (message: string) => {
    // eslint-disable-next-line no-console
    console.error("strims-wasm psi worker:", message);
    worker.terminate();
  };
  worker.onerror = (e: ErrorEvent) => died(e.message || "failed to load");
  worker.onmessage = ({ data }: MessageEvent<PsiWorkerMessage>) => {
    if (data.type === "fatal") died(data.message);
  };
  worker.postMessage(
    { type: "init", role: "psi", options: { logLevel }, port } satisfies InitMessage,
    [port]
  );
  return worker;
};

// Starts a strims-wasm node in a new dedicated worker, and its PSI worker.
// The browser link tests' bare node runs no PSI, and gets no PSI worker.
export const startWasmNode = (options: WasmNodeOptions): WasmNode => {
  const worker = spawnWorker("svc");
  const psi = options.e2e ? undefined : new MessageChannel();
  const psiWorker = psi && startPsiWorker(options.logLevel, psi.port2);
  const frontend = new MessageChannel();
  const signal = new MessageChannel();
  const bus = new PortBus(frontend.port1);
  const peers = new PeerHost((msg, transfer) => signal.port1.postMessage(msg, transfer ?? []));

  const onVisibilityChange = () =>
    signal.port1.postMessage({
      type: "visibility",
      hidden: document.hidden,
      // Locking the screen or backgrounding the tab or PWA on iOS kills open
      // sockets without reliably firing their close events or updating
      // readyState, so every socket is discarded on return.
      forceHUP: IS_IOS,
    } satisfies WindowSignal);
  document.addEventListener("visibilitychange", onVisibilityChange);

  const close = () => {
    document.removeEventListener("visibilitychange", onVisibilityChange);
    signal.port1.onmessage = null;
    signal.port1.close();
    worker.terminate();
    psiWorker?.terminate();
    peers.closeAll();
    bus.shut();
  };

  signal.port1.onmessage = ({ data }: MessageEvent<WorkerSignal>) => {
    if (data.type === "fatal") {
      // eslint-disable-next-line no-console
      console.error("strims-wasm:", data.message);
      close();
      return;
    }
    peers.handle(data);
  };

  // An uncaught error leaves the instance unusable, and a worker that fails
  // to load never posts anything.
  worker.onerror = (e: ErrorEvent) => {
    // eslint-disable-next-line no-console
    console.error("strims-wasm worker:", e.message || "failed to load");
    close();
  };

  const ports = [frontend.port2, signal.port2];
  if (psi) ports.push(psi.port1);
  worker.postMessage(
    {
      type: "init",
      role: "node",
      options,
      frontend: frontend.port2,
      signal: signal.port2,
      psi: psi?.port1,
    } satisfies InitMessage,
    ports
  );

  return { bus, close };
};
