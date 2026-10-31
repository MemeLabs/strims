// Copyright 2022 Strims contributors
// SPDX-License-Identifier: AGPL-3.0-only

declare let self: ServiceWorkerGlobalScope;

// A module's declaration of `self` is its own.
export {};

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
