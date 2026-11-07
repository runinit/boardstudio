(async () => {
  // Pick a release asset from the controlling worker's own cached manifest
  // that THIS page has not requested yet: the offline request must be new.
  const scope = (await navigator.serviceWorker.getRegistration()).scope;
  const names = (await caches.keys()).filter(n => n.startsWith('boardstudio-m1-offline-path-'));
  let manifest = null, cacheName = null;
  for (const name of names) {
    const cache = await caches.open(name);
    const resp = await cache.match(scope + 'service-worker.js');
    const text = resp ? await resp.text() : '';
    const m = text.match(/start_offline_worker\("[^"]+", (\[.*\])\);/s);
    if (m) { manifest = JSON.parse(m[1]); cacheName = name; break; }
  }
  const used = new Set(performance.getEntriesByType('resource').map(e => e.name));
  const candidates = (manifest || []).filter(a => /\.(wasm|json|svg|png|css)$/.test(a)
    && !a.endsWith('service-worker.js') && !used.has(scope + a)).sort();
  const asset = candidates.length ? candidates[candidates.length - 1] : null;
  window.__probeAsset = asset ? scope + asset : null;
  return { cacheName, manifestSize: (manifest || []).length, candidates: candidates.length,
           asset, url: window.__probeAsset, alreadyUsed: asset ? used.has(scope + asset) : null };
})()
