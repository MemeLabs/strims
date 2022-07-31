// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

import React, { useCallback, useEffect } from "react";

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
  const handleSetGetClick = useCallback(() => {
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

  const handleScanPrefixClick = useCallback(async () => {
    await new Promise((resolve, reject) => {
      const req = indexedDB.deleteDatabase("test");
      req.onsuccess = resolve;
      req.onerror = resolve;
      req.onblocked = reject;
    });

    const workerBridge = new WorkerBridge();
    const kv = workerBridge.openKVStore("test", true, false);

    await Promise.all(
      new Array(100).fill(0).map((_, i) => {
        return new Promise((resolve, reject) =>
          kv.put("foo:" + String(i).padStart(5, "0"), new Uint8Array([i]), (error: string) =>
            error ? reject(error) : resolve(undefined)
          )
        );
      })
    );

    const t1 = await new Promise<Uint8Array[]>((resolve, reject) =>
      kv.scanCursor("foo:", "", "foo:00038", 5, 0, (error: string, value: Uint8Array[]) =>
        error ? reject(error) : resolve(value)
      )
    );

    console.log(t1.map((a) => a[0]));
  }, []);

  return (
    <div>
      <Nav />
      <div className="bridge_page">
        <button onClick={handleSetGetClick}>get/set</button>
        <button onClick={handleScanPrefixClick}>scanPrefix</button>
      </div>
    </div>
  );
};

export default BridgePage;
