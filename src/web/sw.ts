declare let self: ServiceWorkerGlobalScope;

import { ResponseMessage } from "../lib/media/hls";

self.addEventListener("fetch", (event: FetchEvent) => {
  const url = new URL(event.request.url);
  const match = url.pathname.match(/_hls-relay\/([^/]+)/);
  if (!match) {
    return;
  }

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
});

// self.addEventListener("fetch", (event: FetchEvent) => {
//   const url = new URL(event.request.url);
//   const match = url.pathname.match(/_cache\/([^/]+)/);
//   if (!match) {
//     return;
//   }

//   event.respondWith(
//     caches.open(match[1]).then((cache) => {
//       return cache.match(event.request);
//     })
//   );
// });
