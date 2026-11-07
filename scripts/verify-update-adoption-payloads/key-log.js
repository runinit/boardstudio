(() => {
  // Diagnostic: record trusted key and click events as they reach the document
  // (capture) and whether anything prevented their default by the time they
  // finish bubbling, so a key that does not activate a control can be explained.
  if (window.__keylogInstalled) { window.__keylog = []; return { installed: true, reset: true }; }
  window.__keylog = [];
  const note = (e) => {
    const t = e.target;
    const entry = { type: e.type, key: e.key, code: e.code,
                    target: t && t.tagName ? ((t.getAttribute && t.getAttribute('aria-label')) || (t.textContent || '').trim().slice(0, 20) || t.tagName) : null,
                    trusted: e.isTrusted };
    window.__keylog.push(entry);
    setTimeout(() => { entry.defaultPrevented = e.defaultPrevented; }, 0);
  };
  ['keydown', 'keyup', 'click'].forEach((type) => document.addEventListener(type, note, true));
  window.__keylogInstalled = true;
  return { installed: true, reset: false };
})()
