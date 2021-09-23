import React, { useEffect } from "react";

import { WindowBridge, WorkerBridge } from "../../lib/bridge";
import Nav from "../components/Nav";

// class MockWorker implements Worker {
//   onerror: ((this: AbstractWorker, ev: ErrorEvent) => any) | null;
//   onmessage: ((this: Worker, ev: MessageEvent) => any) | null;
//   onmessageerror: ((this: Worker, ev: MessageEvent) => any) | null;

//   private channel: MessageChannel;

//   constructor() {
//     this.channel = new MessageChannel();

//     this.channel.port1.onmessage = (...args: any[]) =>
//       void this.onmessage?.apply(this.channel.port1, ...args);
//     this.channel.port1.onmessageerror = (...args: any[]) =>
//       void this.onmessageerror?.apply(this.channel.port1, ...args);
//   }

//   postMessage(...args: any[]) {
//     MessagePort.prototype.postMessage.apply(this.channel.port1, ...args);
//   }

//   terminate(): void {
//     this.channel.port1.close();
//   }

//   addEventListener(...args: any[]) {
//     MessagePort.prototype.addEventListener.apply(this.channel.port1, ...args);
//   }

//   removeEventListener(...args: any[]) {
//     MessagePort.prototype.removeEventListener.apply(this.channel.port1, ...args);
//   }

//   dispatchEvent(...args: any[]): boolean {
//     return MessagePort.prototype.dispatchEvent.apply(this.channel.port1, ...args) as boolean;
//   }
// }

const BridgePage: React.FC = () => {
  useEffect(() => {
    const workerBridge = new WorkerBridge();
    const kv = workerBridge.openKVStore("test", true, false);

    void (async () => {
      const t0 = await new Promise((resolve, reject) =>
        kv.put("foo", new Uint8Array([255, 255, 255, 255]), (error: string) =>
          error ? reject(error) : resolve(undefined)
        )
      );

      const t1 = await new Promise((resolve, reject) =>
        kv.get("foo", (error: string, value: Uint8Array) =>
          error ? reject(error) : resolve(value)
        )
      );

      console.log(t0, t1);
    })();
  }, []);

  return (
    <div>
      <Nav />
      <div>test</div>
    </div>
  );
};

export default BridgePage;
