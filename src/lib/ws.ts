// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

type WSReadWriterEventMap = WebSocketEventMap & {
  "data": Uint8Array;
};

export class WSReadWriter {
  public ws: Promise<WebSocket>;

  constructor(uri: string) {
    const ws = new WebSocket(uri);
    ws.binaryType = "arraybuffer";

    this.ws = new Promise((resolve, reject) => {
      ws.onopen = () => resolve(ws);
      ws.onerror = (e) => console.log(e);
      // ws.onerror = reject;
    });
  }

  public on<K extends keyof WSReadWriterEventMap>(
    method: K,
    handler: (e: WSReadWriterEventMap[K]) => void
  ): void {
    // https://github.com/microsoft/TypeScript/issues/13995
    // eslint-disable-next-line
    const _handler = handler as (e: any) => void;

    void this.ws.then((ws) => {
      if (method === "data") {
        ws.addEventListener("message", (e: MessageEvent<ArrayBuffer>) =>
          _handler(new Uint8Array(e.data))
        );
      } else {
        ws.addEventListener(method, _handler);
      }
    });
  }

  public write(data: Uint8Array): void {
    void this.ws.then((ws) => ws.send(data));
  }

  public close(): void {
    void this.ws.then((ws) => ws.close());
  }
}
