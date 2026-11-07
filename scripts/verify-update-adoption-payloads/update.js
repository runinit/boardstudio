(async () => {
  // Ask the browser to check for the build now served at this origin (the
  // same call a revisit makes), wait for a waiting worker, then poll for the
  // production notice itself: the observer reacts asynchronously.
  const reg = await navigator.serviceWorker.getRegistration();
  try { await reg.update(); } catch (e) { return { updateThrew: String(e).slice(0, 120) }; }
  for (let i = 0; i < 40; i++) {
    await new Promise(r => setTimeout(r, 500));
    if (reg.waiting) break;
  }
  const hasNotice = () => Array.from(document.querySelectorAll('[role=status]'))
    .some(e => (e.textContent || '').includes('Update Available'));
  await new Promise(r => setTimeout(r, 3000));
  for (let i = 0; i < 24 && !hasNotice(); i++) {
    await new Promise(r => setTimeout(r, 500));
  }
  return { updateThrew: null, waiting: reg.waiting ? 'PRESENT' : null, notice: hasNotice() };
})()
