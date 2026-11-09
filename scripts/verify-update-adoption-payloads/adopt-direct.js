(async () => {
  // Bypass the page's own flow: post the adoption message straight to the
  // waiting worker, as a late-opened editor racing a page-side probe would
  // effectively cause. The worker itself must decide.
  const reg = await navigator.serviceWorker.getRegistration();
  if (!reg || !reg.waiting) return { posted: false };
  reg.waiting.postMessage('boardstudio-adopt-waiting-v1');
  await new Promise((r) => setTimeout(r, 4000));
  return { posted: true, waiting: reg.waiting ? reg.waiting.state : null,
           active: reg.active ? reg.active.state : null,
           caches: (await caches.keys()).sort() };
})()
