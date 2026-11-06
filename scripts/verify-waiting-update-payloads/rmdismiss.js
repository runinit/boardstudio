(async () => {
  const reg = await navigator.serviceWorker.getRegistration();
  const notices = Array.from(document.querySelectorAll('[role=status]')).filter(e => (e.textContent||'').includes('Update Available'));
  return {
    loadMark: window.__loadMark || null,
    dismissedUnderReducedMotion: notices.length === 0,
    waiting: reg.waiting ? 'STILL-WAITING' : 'gone',
    controller: navigator.serviceWorker.controller ? 'A-controlled' : null,
    editorAlive: !!document.querySelector('[role=region][aria-label="Layout workspace"]'),
    reduced: matchMedia('(prefers-reduced-motion: reduce)').matches,
  };
})()
