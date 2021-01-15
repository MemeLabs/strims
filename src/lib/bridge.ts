import { EventEmitter } from "events";

declare function postMessage(message: any, targetOrigin?: string, transfer?: any[]): void;
declare function postMessage(message: any, transfer?: any[]): void;

const EVENT_TYPE_OPEN_BUS = 0;
const EVENT_TYPE_OPEN_WEBRTC = 1;
const EVENT_TYPE_OPEN_DATA_CHANNEL = 2;
const EVENT_TYPE_DATA = 3;
const EVENT_TYPE_ERROR = 4;
const EVENT_TYPE_CLOSE = 5;
const EVENT_TYPE_ICE_CANDIDATE = 6;
const EVENT_TYPE_CONNECTION_STATE_CHANGE = 7;
const EVENT_TYPE_ICE_GATHERING_STATE_CHANGE = 8;
const EVENT_TYPE_SIGNALING_STATE_CHANGE = 9;
const EVENT_TYPE_CREATE_OFFER = 10;
const EVENT_TYPE_CREATE_ANSWER = 11;
const EVENT_TYPE_ADD_ICE_CANDIDATE = 12;
const EVENT_TYPE_SET_LOCAL_DESCRIPTION = 13;
const EVENT_TYPE_SET_REMOTE_DESCRIPTION = 14;
const EVENT_TYPE_DATA_CHANNEL = 15;
const EVENT_TYPE_DATA_CHANNEL_DATA = 16;
const EVENT_TYPE_DATA_CHANNEL_OPEN = 17;
const EVENT_TYPE_DATA_CHANNEL_CLOSE = 18;
const EVENT_TYPE_CREATE_DATA_CHANNEL = 19;
const EVENT_TYPE_OPEN_WORKER = 20;

const eventTypeNames = {
  0: "EVENT_TYPE_OPEN_BUS",
  1: "EVENT_TYPE_OPEN_WEBRTC",
  2: "EVENT_TYPE_OPEN_DATA_CHANNEL",
  3: "EVENT_TYPE_DATA",
  4: "EVENT_TYPE_ERROR",
  5: "EVENT_TYPE_CLOSE",
  6: "EVENT_TYPE_ICE_CANDIDATE",
  7: "EVENT_TYPE_CONNECTION_STATE_CHANGE",
  8: "EVENT_TYPE_ICE_GATHERING_STATE_CHANGE",
  9: "EVENT_TYPE_SIGNALING_STATE_CHANGE",
  10: "EVENT_TYPE_CREATE_OFFER",
  11: "EVENT_TYPE_CREATE_ANSWER",
  12: "EVENT_TYPE_ADD_ICE_CANDIDATE",
  13: "EVENT_TYPE_SET_LOCAL_DESCRIPTION",
  14: "EVENT_TYPE_SET_REMOTE_DESCRIPTION",
  15: "EVENT_TYPE_DATA_CHANNEL",
  16: "EVENT_TYPE_DATA_CHANNEL_DATA",
  17: "EVENT_TYPE_DATA_CHANNEL_OPEN",
  18: "EVENT_TYPE_DATA_CHANNEL_CLOSE",
  19: "EVENT_TYPE_CREATE_DATA_CHANNEL",
  20: "EVENT_TYPE_OPEN_WORKER",
};

interface PortState {
  port: MessagePort;
  open: () => void;
}

export class WindowBridge extends EventEmitter {
  private workerConstructor: new () => Worker;
  private nextDataChannelPortId = 0;
  private dataChannelPorts: Map<number, PortState>;

  constructor(workerConstructor: new () => Worker) {
    super();

    this.workerConstructor = workerConstructor;
    this.dataChannelPorts = new Map();

    this.createWorker("default");
  }

  private createWorker(service: string, ...args: any[]): Worker {
    const worker = new this.workerConstructor();
    worker.onmessage = this.handleWorkerMessage.bind(this);
    worker.postMessage({
      service,
      baseURI: location.origin,
      args,
    });
    return worker;
  }

  private handleWorkerMessage({ data }) {
    switch (data.type) {
      case EVENT_TYPE_OPEN_WEBRTC:
        this.openWebRTC(data.port);
        break;
      case EVENT_TYPE_OPEN_DATA_CHANNEL:
        this.openDataChannel(data.port, data.id);
        break;
      case EVENT_TYPE_OPEN_WORKER:
        this.openWorker(data.port, data.service);
        break;
      case EVENT_TYPE_OPEN_BUS:
        this.openBus(data.port, data.label);
        break;
    }
  }

  private openWebRTC(port: MessagePort) {
    // TODO: specify ice servers...
    const peerConnection = new RTCPeerConnection({
      iceServers: [
        {
          urls: ["stun:stun.l.google.com:19302"],
        },
      ],
    });

    const dataChannels: { id: number; port: MessagePort; dataChannel: RTCDataChannel }[] = [];
    const handleDataChannel = (dataChannel) => {
      let open: (...arg: any[]) => void;
      const ready = new Promise((resolve) => (open = resolve));

      const { port1, port2 } = new MessageChannel();
      const portId = this.nextDataChannelPortId++;
      this.dataChannelPorts.set(portId, { port: port2, open });

      port.postMessage({
        type: EVENT_TYPE_DATA_CHANNEL,
        id: portId,
        label: dataChannel.label,
      });

      dataChannel.binaryType = "arraybuffer";

      dataChannel.onmessage = (e: MessageEvent) =>
        ready.then(() =>
          port1.postMessage(
            {
              type: EVENT_TYPE_DATA_CHANNEL_DATA,
              timestamp: Date.now(),
              data: e.data,
            },
            [e.data]
          )
        );

      dataChannel.onopen = () =>
        ready.then(() =>
          port1.postMessage({
            type: EVENT_TYPE_DATA_CHANNEL_OPEN,
          })
        );

      port1.onmessage = ({ data }) =>
        ready.then(() => {
          // console.log("window data channel event", data);
          switch (data.type) {
            case EVENT_TYPE_DATA_CHANNEL_DATA:
              dataChannel.send(data.data);
              break;
            case EVENT_TYPE_DATA_CHANNEL_CLOSE:
              dataChannel.close();
              break;
          }
        });

      dataChannels.push({
        port: port1,
        id: portId,
        dataChannel,
      });
    };

    peerConnection.ondatachannel = ({ channel }: RTCDataChannelEvent) => handleDataChannel(channel);

    const onclose = () => {
      dataChannels.forEach(({ id, port, dataChannel }) => {
        port.postMessage({ type: EVENT_TYPE_DATA_CHANNEL_CLOSE });
        this.dataChannelPorts.delete(id);

        dataChannel.onmessage = null;
        dataChannel.onopen = null;
        dataChannel.close();

        port.onmessage = null;
        port.close();
      });

      peerConnection.ondatachannel = null;
      peerConnection.onicecandidate = null;
      peerConnection.onconnectionstatechange = null;
      peerConnection.onicegatheringstatechange = null;
      peerConnection.onsignalingstatechange = null;
      peerConnection.close();

      port.onmessage = null;
      port.close();
    };

    peerConnection.onicecandidate = (e: RTCPeerConnectionIceEvent) =>
      port.postMessage({
        type: EVENT_TYPE_ICE_CANDIDATE,
        candidate: JSON.stringify(e.candidate),
      });

    peerConnection.onconnectionstatechange = () => {
      const state = peerConnection.iceConnectionState;
      port.postMessage({
        type: EVENT_TYPE_CONNECTION_STATE_CHANGE,
        state,
      });

      if (state === "failed" || state === "disconnected" || state === "closed") {
        onclose();
      }
    };

    peerConnection.onicegatheringstatechange = () =>
      port.postMessage({
        type: EVENT_TYPE_ICE_GATHERING_STATE_CHANGE,
        state: peerConnection.iceGatheringState,
      });

    peerConnection.onsignalingstatechange = () =>
      port.postMessage({
        type: EVENT_TYPE_SIGNALING_STATE_CHANGE,
        state: peerConnection.signalingState,
      });

    port.onmessage = ({ data }) => {
      // console.log("window event", eventTypeNames[data.type], data);
      switch (data.type) {
        case EVENT_TYPE_CREATE_OFFER:
          peerConnection
            .createOffer()
            .then((description) =>
              port.postMessage({
                type: EVENT_TYPE_CREATE_OFFER,
                description: JSON.stringify(description),
              })
            )
            .catch((error) => {
              port.postMessage({
                type: EVENT_TYPE_CREATE_OFFER,
                error: String(error),
              });
              onclose();
            });
          break;
        case EVENT_TYPE_CREATE_ANSWER:
          peerConnection
            .createAnswer()
            .then((description) =>
              port.postMessage({
                type: EVENT_TYPE_CREATE_ANSWER,
                description: JSON.stringify(description),
              })
            )
            .catch((error) => {
              port.postMessage({
                type: EVENT_TYPE_CREATE_ANSWER,
                error: String(error),
              });
              onclose();
            });
          break;
        case EVENT_TYPE_CREATE_DATA_CHANNEL:
          handleDataChannel(peerConnection.createDataChannel(data.label));
          break;
        case EVENT_TYPE_ADD_ICE_CANDIDATE:
          peerConnection.addIceCandidate(new RTCIceCandidate(JSON.parse(data.candidate)));
          break;
        case EVENT_TYPE_SET_LOCAL_DESCRIPTION:
          peerConnection.setLocalDescription(
            new RTCSessionDescription(JSON.parse(data.description))
          );
          break;
        case EVENT_TYPE_SET_REMOTE_DESCRIPTION:
          peerConnection.setRemoteDescription(
            new RTCSessionDescription(JSON.parse(data.description))
          );
          break;
        case EVENT_TYPE_CLOSE:
          peerConnection.close();
          onclose();
          break;
      }
    };
  }

  private openDataChannel(port: MessagePort, id: number) {
    const portState = this.dataChannelPorts.get(id);
    if (portState === undefined) {
      port.postMessage(undefined);
      return;
    }
    this.dataChannelPorts.delete(id);

    port.postMessage(portState.port, [portState.port]);
    setTimeout(() => portState.open(), 100);
  }

  private openWorker(port: MessagePort, service: string) {
    const worker = this.createWorker(service);
    this.once(`busport:${service}`, (p: MessagePort) => port.postMessage({ port: p }, [p]));

    port.onmessage = ({ data }) => {
      switch (data.type) {
        case EVENT_TYPE_CLOSE:
          worker.terminate();
          break;
      }
    };
  }

  private openBus(port: MessagePort, label: string) {
    this.emit(`busport:${label}`, port);
    this.emit(`busopen:${label}`, new Bus(port, label));
  }
}

export class Bus extends EventEmitter {
  public port: MessagePort;
  public label: string;

  constructor(port: MessagePort, label) {
    super();

    this.port = port;
    this.label = label;

    port.onmessage = ({ data }) => {
      switch (data.type) {
        case EVENT_TYPE_DATA:
          this.emit("data", data.data);
          break;
        case EVENT_TYPE_CLOSE:
          this.emit("close");
          break;
      }
    };
  }

  public write(data: ArrayBuffer | Uint8Array) {
    const buffer = data instanceof ArrayBuffer ? data : data.buffer;

    this.port.postMessage(
      {
        type: EVENT_TYPE_DATA,
        data: buffer,
        timestamp: Date.now(),
      },
      [buffer]
    );
  }

  public close() {
    this.port.postMessage({ type: EVENT_TYPE_CLOSE });
    this.port.close();
  }
}

export interface WebSocketProxy {
  ondata(data: Uint8Array, n: number, timestamp: number): void;
  onopen(): void;
  onclose(): void;
  onerror(message: string): void;
}

export interface WebRTCProxy {
  onicecandidate(candidate: string): void;
  onconnectionstatechange(state: string): void;
  onicegatheringstatechange(state: string): void;
  onsignalingstatechange(state: string): void;
  oncreateoffer(error: string, description: string): void;
  oncreateanswer(error: string, description: string): void;
  ondatachannel(id: number, label: string): void;
}

export interface DataChannelProxy {
  onerror(message: string): void;
  ondata(data: Uint8Array, n: number, timestamp: number): void;
  onclose(): void;
  onopen(): void;
}

export interface ServiceProxy {
  openBus(any): DataChannelProxy;
}

export class WorkerBridge {
  public openWebSocket(uri: string, proxy: WebSocketProxy) {
    const ws = new WebSocket(uri);

    const onclose = () => {
      ws.onopen = null;
      ws.onclose = null;
      ws.onerror = null;
      ws.onmessage = null;
    };

    ws.binaryType = "arraybuffer";
    ws.onopen = () => proxy.onopen();
    ws.onclose = () => {
      onclose();
      proxy.onclose();
    };
    ws.onerror = (e: ErrorEvent) => proxy.onerror(String(e.message || "unknown websocket error"));
    ws.onmessage = ({ data }) => proxy.ondata(new Uint8Array(data), data.byteLength, Date.now());

    return {
      write: (data: Uint8Array) => ws.send(data),
      close: () => {
        onclose();
        ws.close();
      },
    };
  }

  public openWebRTC(proxy: WebRTCProxy) {
    const { port1, port2 } = new MessageChannel();
    port1.onmessage = ({ data }) => {
      // console.log("worker event", eventTypeNames[data.type], data);
      switch (data.type) {
        case EVENT_TYPE_ICE_CANDIDATE:
          proxy.onicecandidate(data.candidate);
          break;
        case EVENT_TYPE_CONNECTION_STATE_CHANGE:
          proxy.onconnectionstatechange(data.state);
          break;
        case EVENT_TYPE_ICE_GATHERING_STATE_CHANGE:
          proxy.onicegatheringstatechange(data.state);
          break;
        case EVENT_TYPE_SIGNALING_STATE_CHANGE:
          proxy.onsignalingstatechange(data.state);
          break;
        case EVENT_TYPE_CREATE_OFFER:
          proxy.oncreateoffer(data.error, data.description);
          break;
        case EVENT_TYPE_CREATE_ANSWER:
          proxy.oncreateanswer(data.error, data.description);
          break;
        case EVENT_TYPE_DATA_CHANNEL:
          proxy.ondatachannel(data.id, data.label);
          break;
      }
    };

    postMessage(
      {
        type: EVENT_TYPE_OPEN_WEBRTC,
        port: port2,
      },
      [port2]
    );

    return {
      createOffer: () =>
        port1.postMessage({
          type: EVENT_TYPE_CREATE_OFFER,
        }),
      createAnswer: () =>
        port1.postMessage({
          type: EVENT_TYPE_CREATE_ANSWER,
        }),
      createDataChannel: (label: string) =>
        port1.postMessage({
          type: EVENT_TYPE_CREATE_DATA_CHANNEL,
          label,
        }),
      addIceCandidate: (candidate: string) =>
        port1.postMessage({
          type: EVENT_TYPE_ADD_ICE_CANDIDATE,
          candidate,
        }),
      setLocalDescription: (description: string) =>
        port1.postMessage({
          type: EVENT_TYPE_SET_LOCAL_DESCRIPTION,
          description,
        }),
      setRemoteDescription: (description: string) =>
        port1.postMessage({
          type: EVENT_TYPE_SET_REMOTE_DESCRIPTION,
          description,
        }),
      close: () => {
        port1.postMessage({
          type: EVENT_TYPE_CLOSE,
        });
        port1.close();
      },
    };
  }

  // openDataChannel opens a data channel created by a call to openWebRTC. This
  // allows multiple workers to share an RTCPeerConnection.
  public openDataChannel(id: number, proxy: DataChannelProxy) {
    const { port1, port2 } = new MessageChannel();

    const ready = new Promise<MessagePort>((resolve, reject) => {
      port1.onmessage = ({ data: port }) => {
        port1.close();

        if (port === undefined) {
          const err = new Error("data channel invalid or in use");
          proxy.onerror(err.message);
          reject(err);
          return;
        }
        resolve(port);

        port.onmessage = ({ data }) => {
          // console.log("worker data channel event", data);
          switch (data.type) {
            case EVENT_TYPE_DATA_CHANNEL_DATA:
              proxy.ondata(new Uint8Array(data.data), data.data.byteLength, data.timestamp);
              break;
            case EVENT_TYPE_DATA_CHANNEL_OPEN:
              proxy.onopen();
              break;
            case EVENT_TYPE_DATA_CHANNEL_CLOSE:
              proxy.onclose();
              break;
          }
        };
      };
    });

    postMessage(
      {
        type: EVENT_TYPE_OPEN_DATA_CHANNEL,
        port: port2,
        id,
      },
      [port2]
    );

    return {
      write: ({ buffer }: Uint8Array) =>
        ready.then((port) =>
          port.postMessage(
            {
              type: EVENT_TYPE_DATA_CHANNEL_DATA,
              data: buffer,
            },
            [buffer]
          )
        ),
      close: () =>
        ready.then((port) =>
          port.postMessage({
            type: EVENT_TYPE_DATA_CHANNEL_CLOSE,
          })
        ),
    };
  }

  public openWorker(service: string, proxy: ServiceProxy) {
    const { port1, port2 } = new MessageChannel();

    port1.onmessage = ({ data: { port } }) => {
      const bus = proxy.openBus({
        write: ({ buffer }: Uint8Array) =>
          port.postMessage(
            {
              type: EVENT_TYPE_DATA,
              data: buffer,
              timestamp: Date.now(),
            },
            [buffer]
          ),
        close: () => {
          port.postMessage({ type: EVENT_TYPE_CLOSE });
          port1.postMessage({ type: EVENT_TYPE_CLOSE });
        },
      });

      port.onmessage = ({ data }) => {
        switch (data.type) {
          case EVENT_TYPE_DATA:
            bus.ondata(new Uint8Array(data.data), data.data.byteLength, data.timestamp);
            break;
          case EVENT_TYPE_CLOSE:
            bus.onclose();
            break;
        }
      };
    };

    postMessage(
      {
        type: EVENT_TYPE_OPEN_WORKER,
        port: port2,
        service,
      },
      [port2]
    );
  }

  public openBus(label: string, proxy: DataChannelProxy) {
    const { port1, port2 } = new MessageChannel();

    port1.onmessage = ({ data }) => {
      switch (data.type) {
        case EVENT_TYPE_DATA:
          proxy.ondata(new Uint8Array(data.data), data.data.byteLength, data.timestamp);
          break;
        case EVENT_TYPE_CLOSE:
          proxy.onclose();
          break;
      }
    };

    postMessage(
      {
        type: EVENT_TYPE_OPEN_BUS,
        port: port2,
        label,
      },
      [port2]
    );

    return {
      write: ({ buffer }: Uint8Array) =>
        port1.postMessage(
          {
            type: EVENT_TYPE_DATA,
            data: buffer,
            timestamp: Date.now(),
          },
          [buffer]
        ),
      close: () => port1.postMessage({ type: EVENT_TYPE_CLOSE }),
    };
  }

  public openKVStore(name: string, createTable: boolean, readOnly: boolean) {
    const openReq = indexedDB.open(name);
    openReq.onupgradeneeded = (e: any) => {
      if (!createTable) {
        e.target.transaction.abort();
        return;
      }
      openReq.result.createObjectStore("data");
    };

    const txReady: Promise<IDBTransaction> = new Promise((resolve, reject) => {
      const mode = readOnly ? "readonly" : "readwrite";
      openReq.onsuccess = () => resolve(openReq.result.transaction(["data"], mode));
      openReq.onerror = reject;
      openReq.onblocked = reject;
    });

    const transact = <T>(
      operator: (db: IDBObjectStore) => IDBRequest<T>,
      done: (error: string | null, value?: T) => void
    ) => {
      txReady
        .then((tx: IDBTransaction) => {
          const req = operator(tx.objectStore("data"));
          req.onsuccess = () => done(null, req.result);
          req.onerror = (e) => done(String(e));
        })
        .catch((e) => done(String(e.message || "unknown storage error")));
    };

    return {
      put: (key: string, value: Uint8Array, done: (error: string | null) => void) => {
        transact((s) => s.put(value, key), done);
      },
      delete: (key: string, done: (error: string | null) => void) => {
        transact((s) => s.delete(key), done);
      },
      get: (key: string, done: (error: string | null, value?: Uint8Array) => void) => {
        transact((s) => s.get(key), done);
      },
      scanPrefix: (prefix: string, done: (error: string | null, value?: any[]) => void) => {
        const range = IDBKeyRange.bound(prefix, prefix + "\uffff", false, false);
        transact((s) => s.getAll(range), done);
      },
      rollback: (done: (error: string | null) => void) => {
        txReady
          .then((tx: IDBTransaction) => {
            tx.onabort = () => done(null);
            tx.onerror = (e) => done(String(e));
            tx.abort();
          })
          .catch((e) => done(String(e)));
      },
      commit: (done: (error: string | null) => void) => {
        txReady
          .then((tx: IDBTransaction) => {
            tx.oncomplete = () => done(null);
            tx.onerror = (e) => done(String(e));
            (tx as any).commit?.();
          })
          .catch((e) => done(String(e)));
      },
    };
  }

  public deleteKVStore(name: string, done: (err: string | null) => void) {
    const res = indexedDB.deleteDatabase(name);
    res.onsuccess = () => done(null);
    res.onerror = (e) => done(String(e));
  }

  public syncLogs(data: Uint8Array) {
    const { level, caller, msg, ...args } = JSON.parse(new TextDecoder().decode(data));
    // eslint-disable-next-line
    console.log(
      level.toUpperCase().padEnd(10),
      caller.padEnd(30),
      msg,
      Object.keys(args).length === 0 ? "" : args
    );
  }
}
