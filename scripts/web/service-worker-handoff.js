/* Deployment bridge for an existing classic or module worker registration.
 * No fetch handler: the replacement shell must come from the deployed server.
 * Neither project storage nor any application's caches are changed here.
 */
self.addEventListener("install", (event) => {
  event.waitUntil(self.skipWaiting());
});

self.addEventListener("activate", (event) => {
  event.waitUntil((async () => {
    await self.clients.claim();
    const scope = new URL(self.registration.scope);
    // Default includeUncontrolled=false excludes clients of other registrations,
    // including a more specific application scope on the same origin.
    const clients = await self.clients.matchAll({ type: "window" });
    await Promise.allSettled(clients.filter((client) => {
      const url = new URL(client.url);
      return url.origin === scope.origin && url.pathname.startsWith(scope.pathname);
    }).map((client) => client.navigate(client.url)));
  })());
});
