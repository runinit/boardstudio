(async () => {
  // Bind each versioned offline cache to the release whose worker bootstrap
  // text it stores (the worker caches its own service-worker.js).
  const out = {};
  const shells = {};
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
      // Which build's shell does this cache hold? (stamped meta in its index.html)
      const idx = keys.find(k => k.url.endsWith('/index.html')) || null;
      const idxResp = idx ? await cache.match(idx) : null;
      const idxText = idxResp ? await idxResp.text() : '';
      const sm = idxText.match(/name="boardstudio-build" content="([^"]+)"/);
      shells[name] = sm ? sm[1] : (idx ? 'UNSTAMPED' : 'NO_INDEX');
    } catch (e) { out[name] = 'ERROR ' + String(e).slice(0, 80); }
  }
  return { loadMark: window.__loadMark || null, cachedWorkerVersions: out, cachedShellBuilds: shells };
})()
