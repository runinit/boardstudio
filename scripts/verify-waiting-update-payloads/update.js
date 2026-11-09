(async () => {
  const reg = await navigator.serviceWorker.getRegistration();
  try { await reg.update(); } catch (e) { return { updateThrew: String(e).slice(0,120) }; }
  for (let i = 0; i < 40; i++) {
    await new Promise(r => setTimeout(r, 500));
    if (reg.waiting) break;
  }
  // Settle then poll for the notice itself: the production observer reacts
  // to the waiting state asynchronously, so asserting immediately after
  // install would race the toast. Absence after this window is a real
  // negative; presence is a real positive.
  const hasNotice = () => Array.from(document.querySelectorAll('[role=status]'))
    .some(e => (e.textContent||'').includes('Update Available'));
  await new Promise(r => setTimeout(r, 3000));
  for (let i = 0; i < 24 && !hasNotice(); i++) {
    await new Promise(r => setTimeout(r, 500));
  }
  const all = Array.from(document.querySelectorAll('[role=status]'));
  const toast = all.find(e => (e.textContent||'').includes('Update Available')) || null;
  const statuses = all.map(e => (e.textContent||'').slice(0,60));
  const cachesList = await caches.keys();
  const out = {
    loadMark: window.__loadMark || null,
    waiting: reg.waiting ? 'PRESENT' : null,
    controller: navigator.serviceWorker.controller ? navigator.serviceWorker.controller.scriptURL : null,
    updateNotice: statuses.some(t => t.includes('Update Available')),
    allStatuses: statuses,
    editorAlive: !!document.querySelector('[role=region][aria-label="Layout workspace"]'),
    caches: cachesList,
  };
  if (toast) {
    const close = toast.querySelector('button');
    out.toastRole = toast.getAttribute('role');
    out.toastAtomic = toast.getAttribute('aria-atomic');
    out.toastDataType = toast.getAttribute('data-type');
    out.closeTabIndex = close ? close.tabIndex : null;
    out.regionText = document.querySelector('[aria-label$="notifications"]')?.textContent?.slice(0,40);
  }
  return out;
})()
