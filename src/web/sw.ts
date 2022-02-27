self.addEventListener("fetch", (event: FetchEvent) => {
  const url = new URL(event.request.url);
  const match = url.pathname.match(/_cache\/([^/]+)/);
  if (!match) {
    return;
  }

  event.respondWith(
    caches.open(match[1]).then((cache) => {
      return cache.match(event.request);
    })
  );
});
