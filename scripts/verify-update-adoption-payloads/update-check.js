(async () => {
  // Quick update check for the no-build control: ask for an update, report
  // waiting/notice immediately without the long waits in update.js (which
  // expects a published B). Never adopts, only reports.
  const reg = await navigator.serviceWorker.getRegistration();
  try { await reg.update(); } catch (e) { return { updateThrew: String(e).slice(0, 120) }; }
  await new Promise(r => setTimeout(r, 2000));
  const hasNotice = () => Array.from(document.querySelectorAll('[role=status]'))
    .some(e => (e.textContent || '').includes('Update Available'));
  return { updateThrew: null, waiting: reg.waiting ? 'PRESENT' : null, notice: hasNotice() };
})()
