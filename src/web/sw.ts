// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

declare let self: ServiceWorkerGlobalScope;

import { ResponseMessage } from "../lib/media/hls";

const handleHLSRelayRequest = (event: FetchEvent, url: URL) => {
  event.respondWith(
    new Promise<Response>((resolve) => {
      const { port1, port2 } = new MessageChannel();

      const id = setTimeout(() => {
        resolve(new Response("timeout", { status: 408 }));
        port1.close();
      }, 1000);

      port1.onmessage = (event: MessageEvent<ResponseMessage>) => {
        clearTimeout(id);
        resolve(new Response(event.data.body));
        port1.close();
      };

      void self.clients.get(event.clientId).then((client) => {
        client.postMessage(
          {
            type: "HLS_RELAY_REQUEST",
            url: url.pathname,
            port: port2,
          },
          [port2]
        );
      });
    })
  );
};

const storeInCache = async (req: URL | RequestInfo, res: Response) => {
  const cache = await caches.open(`${GIT_HASH}_static`);
  await cache.put(req, res);
};

const cacheFirst = async (req: URL | RequestInfo) => {
  let res = await caches.match(req);
  if (!res) {
    res = await fetch(req);
    void storeInCache(req, res.clone());
  }
  return res;
};

const handleStaticRequest = (event: FetchEvent) => {
  if (IS_PRODUCTION) {
    event.respondWith(cacheFirst(event.request));
  }
};

const routes: [RegExp, (event: FetchEvent, url: URL) => void][] = [
  [/_hls-relay\/([^/]+)/, handleHLSRelayRequest],
  [/\.(css|js|json|png|svg|wasm|ttf|ico)$/, handleStaticRequest],
];

self.addEventListener("fetch", (event: FetchEvent) => {
  const url = new URL(event.request.url);

  if (url.protocol !== "https:") {
    return;
  }

  if (event.request.referrer) {
    const referrer = new URL(event.request.referrer);
    if (url.origin !== referrer.origin) {
      return;
    }
  }

  for (const [route, handler] of routes) {
    if (url.pathname.match(route)) {
      return handler(event, url);
    }
  }
});

self.addEventListener("install", (event) => {
  event.waitUntil(self.skipWaiting());
});

const deleteStaleCaches = async () => {
  const keys = await caches.keys();
  const expired = keys.filter((k) => !k.startsWith(GIT_HASH));
  await Promise.all(expired.map((k) => caches.delete(k)));
};

self.addEventListener("activate", (event) => {
  event.waitUntil(self.clients.claim());
  event.waitUntil(deleteStaleCaches());
});
