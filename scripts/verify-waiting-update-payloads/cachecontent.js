(async () => {
  const out = {};
  for (const name of await caches.keys()) {
    try {
      const cache = await caches.open(name);
      const keys = await cache.keys();
      const swKey = keys.find(k => k.url.endsWith('/service-worker.js')) || null;
      if (!swKey) { out[name] = 'NO_SW_ENTRY'; continue; }
      const resp = await cache.match(swKey);
      const text = resp ? await resp.text() : null;
      const m = text ? text.match(/start_offline_worker\("([^"]+)"/) : null;
      out[name] = m ? m[1] : (text ? text.slice(0, 100) : 'EMPTY');
    } catch (e) { out[name] = 'ERROR ' + String(e).slice(0, 80); }
  }
  return { loadMark: window.__loadMark || null, cachedWorkerVersions: out };
})()
