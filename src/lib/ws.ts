export class WSReadWriter {
  public ws: Promise<WebSocket>;

  constructor(uri: string) {
    const ws = new WebSocket(uri);
    // set binaryType to arraybuffer over default Buffer objects
    ws.binaryType = "arraybuffer";

    this.ws = new Promise((resolve, reject) => {
      ws.onopen = () => resolve(ws);
      ws.onerror = reject;
    });
  }

  public on(method: string, handler: (...args: any[]) => any) {
    this.ws.then((ws) => {
      if (method === "data") {
        // add event listener to 'data' event
        ws.addEventListener("message", (e) => handler(new Uint8Array(e.data)));
      } else {
        ws.addEventListener(method, handler);
      }
    });
  }

  public write(data: Uint8Array) {
    this.ws.then((ws) => ws.send(data));
  }
}
