(async () => {
  // Offline-continuity probe: the origin may be stopped while this runs, so a
  // 200 can only come from the controlling worker's cache.
  const url = window.__probeAsset;
  if (!url) return { fetched: false, reason: 'no probe asset' };
  try {
    const resp = await fetch(url);
    const bytes = new Uint8Array(await resp.arrayBuffer());
    const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
    const sha = Array.from(digest).map(b => b.toString(16).padStart(2, '0')).join('');
    return { fetched: true, url, status: resp.status, bytes: bytes.length, sha256: sha,
             loadMark: window.__loadMark || null };
  } catch (e) {
    return { fetched: false, url, error: String(e).slice(0, 120), loadMark: window.__loadMark || null };
  }
})()
