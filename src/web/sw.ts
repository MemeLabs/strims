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

const handleSVCWorkerRequest = (event: FetchEvent) => {
  event.respondWith(
    caches.match(event.request).then(async (res) => {
      if (!res) {
        res = await fetch(event.request.clone());
        const cache = await caches.open(`${GIT_HASH}_svc`);
        await cache.put(event.request, res.clone());
      }
      return res;
    })
  );
};

const routes: [RegExp, (event: FetchEvent, url: URL) => void][] = [
  [/_hls-relay\/([^/]+)/, handleHLSRelayRequest],
  [/svc\.([a-f0-9]+)\.wasm/, handleSVCWorkerRequest],
];

self.addEventListener("fetch", (event: FetchEvent) => {
  const url = new URL(event.request.url);
  for (const [route, handler] of routes) {
    if (url.pathname.match(route)) {
      return handler(event, url);
    }
  }
});

self.addEventListener("activate", (event) => {
  event.waitUntil(async () => {
    const keys = await caches.keys();
    const expired = keys.filter((k) => !k.startsWith(GIT_HASH));
    await Promise.all(expired.map((k) => caches.delete(k)));
  });
});
