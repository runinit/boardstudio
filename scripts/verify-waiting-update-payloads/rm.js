(async () => {
  const reg = await navigator.serviceWorker.getRegistration();
  try { await reg.update(); } catch (e) { return { updateThrew: String(e).slice(0,120) }; }
  for (let i = 0; i < 40; i++) {
    await new Promise(r => setTimeout(r, 500));
    if (reg.waiting) break;
  }
  // Settle then poll for the notice itself: the production observer reacts
  // to the waiting state asynchronously, so reading immediately after
  // install would race the toast.
  const hasNotice = () => Array.from(document.querySelectorAll('[role=status]'))
    .some(e => (e.textContent||'').includes('Update Available'));
  await new Promise(r => setTimeout(r, 3000));
  for (let i = 0; i < 24 && !hasNotice(); i++) {
    await new Promise(r => setTimeout(r, 500));
  }
  const toast = Array.from(document.querySelectorAll('[role=status]')).find(e => (e.textContent||'').includes('Update Available')) || null;
  if (!toast) return { notice: false, reduced: matchMedia('(prefers-reduced-motion: reduce)').matches, waiting: reg.waiting ? 'PRESENT' : null, loadMark: window.__loadMark || null };
  const cs = getComputedStyle(toast);
  const close = toast.querySelector('button');
  const ccs = close ? getComputedStyle(close) : null;
  const cachesList = await caches.keys();
  return {
    notice: true,
    reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
    loadMark: window.__loadMark || null,
    waiting: reg.waiting ? 'PRESENT' : null,
    controller: navigator.serviceWorker.controller ? 'A-controlled' : null,
    editorAlive: !!document.querySelector('[role=region][aria-label="Layout workspace"]'),
    toastComputed: { animationName: cs.animationName, animationDuration: cs.animationDuration, transitionProperty: cs.transitionProperty, transitionDuration: cs.transitionDuration },
    closeComputed: ccs ? { animationName: ccs.animationName, transitionDuration: ccs.transitionDuration } : null,
    closeTabIndex: close ? close.tabIndex : null,
    caches: cachesList,
  };
})()
