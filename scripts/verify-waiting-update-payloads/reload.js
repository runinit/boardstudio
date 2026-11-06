(async () => {
  const reg = await navigator.serviceWorker.getRegistration();
  const hasNotice = Array.from(document.querySelectorAll('[role=status]')).some(e => (e.textContent||'').includes('Update Available'));
  const cachesList = await caches.keys();
  return {
    reappearedAfterReload: hasNotice,
    waiting: reg.waiting ? 'STILL-WAITING' : 'adopted-B',
    loadMark: window.__loadMark === undefined ? 'marker-gone (fresh context, expected)' : window.__loadMark,
    caches: cachesList,
  };
})()
