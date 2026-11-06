(async () => {
  const reg = await navigator.serviceWorker.getRegistration();
  let threw = false;
  try { await reg.update(); } catch (e) { threw = true; }
  await new Promise(r => setTimeout(r, 3000));
  const hasNotice = Array.from(document.querySelectorAll('[role=status]')).some(e => (e.textContent||'').includes('Update Available'));
  return {
    loadMark: window.__loadMark || null,
    updateThrew: threw,
    installing: reg.installing ? reg.installing.state : null,
    waiting: reg.waiting ? 'PRESENT' : null,
    failedControlNoNotice: !hasNotice,
    controller: navigator.serviceWorker.controller ? 'A-controlled' : null,
    editorAlive: !!document.querySelector('[role=region][aria-label="Layout workspace"]'),
  };
})()
