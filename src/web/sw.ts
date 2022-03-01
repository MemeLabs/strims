import { MessageData } from "../lib/media/hls";

self.addEventListener("fetch", (event: FetchEvent) => {
  const url = new URL(event.request.url);
  const match = url.pathname.match(/_hls-relay\/([^/]+)/);
  if (!match) {
    return;
  }

  event.respondWith(
    new Promise<Response>((resolve) => {
      const ch = new BroadcastChannel(match[1]);

      const id = setTimeout(() => {
        resolve(new Response("timeout", { status: 408 }));
        ch.close();
      }, 1000);

      ch.onmessage = (event: MessageEvent<MessageData>) => {
        if (event.data.type === "RESPONSE" && event.data.url === url.pathname) {
          clearTimeout(id);
          resolve(new Response(event.data.body));
          ch.close();
        }
      };

      ch.postMessage({ type: "REQUEST", url: url.pathname });
    })
  );
});
