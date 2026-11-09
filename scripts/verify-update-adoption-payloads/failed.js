(async () => {
  // Control for a failed installation. Ask for the update, then poll fast for
  // the whole attempt: the notice or any Update Now control must never be
  // visible while the install runs or after it fails.
  const reg = await navigator.serviceWorker.getRegistration();
  const controlsNow = () => !!document.querySelector("[aria-label='Update Now']")
    || Array.from(document.querySelectorAll('[role=status]')).some(e => (e.textContent || '').includes('Update Available'));
  let threw = false, seenInstalling = false, controlsEverShown = false;
  const attempt = reg.update().catch(() => { threw = true; });
  const t0 = Date.now();
  while (Date.now() - t0 < 8000) {
    if (reg.installing) seenInstalling = true;
    if (controlsNow()) controlsEverShown = true;
    await new Promise(r => setTimeout(r, 50));
  }
  await attempt;
  return {
    updateThrew: threw,
    seenInstalling,
    controlsEverShown,
    installing: reg.installing ? reg.installing.state : null,
    waiting: reg.waiting ? 'PRESENT' : null,
  };
})()
