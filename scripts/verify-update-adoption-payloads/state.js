(async () => {
  // Read-only observation of the loaded page and its registration. Never
  // calls update(), never clicks: it only reports what production exposes.
  const reg = await navigator.serviceWorker.getRegistration();
  const q = (s) => document.querySelector(s);
  const statuses = Array.from(document.querySelectorAll('[role=status]')).map(e => (e.textContent || '').slice(0, 60));
  const report = Array.from(document.querySelectorAll('p[role=alert], p[role=status]')).map(e => (e.textContent || '').slice(0, 200));
  const dialog = q('[role=alertdialog]');
  const nav = performance.getEntriesByType('navigation')[0];
  const save = q('[aria-label^="Local save"]');
  return {
    loadMark: window.__loadMark || null,
    build: (q('meta[name=boardstudio-build]') || {}).content || null,
    href: location.href,
    scope: reg ? reg.scope : null,
    focusOnUpdateNow: !!(document.activeElement && document.activeElement.getAttribute('aria-label') === 'Update Now'),
    navType: nav ? nav.type : null,
    controller: navigator.serviceWorker.controller ? navigator.serviceWorker.controller.scriptURL : null,
    active: reg && reg.active ? reg.active.state : null,
    waiting: reg && reg.waiting ? reg.waiting.state : null,
    installing: reg && reg.installing ? reg.installing.state : null,
    updateNotice: statuses.some(t => t.includes('Update Available')),
    allStatuses: statuses,
    report,
    toast: !!q('[data-slot=toast]'),
    toastUpdateNow: !!q("[data-slot=toast-provider] [aria-label='Update Now']"),
    headerUpdateNow: !!q(".header-update [aria-label='Update Now']"),
    headerNotice: !!q('.header-update'),
    dialogTitle: dialog ? ((dialog.querySelector('[data-slot=alert-dialog-title], h2') || {}).textContent || null) : null,
    dialogText: dialog ? (dialog.textContent || '').slice(0, 400) : null,
    saveLabel: save ? save.getAttribute('aria-label') : null,
    editorAlive: !!q('[role=region][aria-label="Layout workspace"]'),
    mainBoard: (document.body.textContent || '').includes('Main board'),
    library: (document.body.textContent || '').includes('Your keyboards'),
    caches: (await caches.keys()).sort(),
  };
})()
