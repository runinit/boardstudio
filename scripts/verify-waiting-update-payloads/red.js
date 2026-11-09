(async () => {
  const reg = await navigator.serviceWorker.getRegistration();
  const statuses = Array.from(document.querySelectorAll('[role=status]')).map(e => (e.textContent||'').slice(0,60));
  const cachesList = await caches.keys();
  return {
    loadMark: window.__loadMark || null,
    controller: navigator.serviceWorker.controller ? navigator.serviceWorker.controller.scriptURL : null,
    waiting: reg && reg.waiting ? 'PRESENT' : null,
    installing: reg && reg.installing ? reg.installing.state : null,
    updateNotice: statuses.some(t => t.includes('Update Available')),
    allStatuses: statuses,
    editorAlive: !!document.querySelector('[role=region][aria-label="Layout workspace"]'),
    caches: cachesList,
  };
})()
