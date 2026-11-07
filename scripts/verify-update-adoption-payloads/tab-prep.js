(() => {
  // Real Tab order: focus the control that immediately precedes the toast's
  // Update Now in the document's tab sequence, so ONE trusted Tab keypress must
  // land on it. Also reports positive tabindex values that would reorder it.
  const sel = 'a[href],button:not([disabled]),input:not([disabled]):not([type=hidden]),select:not([disabled]),textarea:not([disabled]),[tabindex]:not([tabindex="-1"])';
  const visible = (e) => {
    const r = e.getBoundingClientRect();
    const cs = getComputedStyle(e);
    return r.width > 0 && r.height > 0 && cs.visibility !== 'hidden' && cs.display !== 'none';
  };
  const list = Array.from(document.querySelectorAll(sel)).filter(visible);
  const positive = list.filter((e) => e.tabIndex > 0).length;
  const target = document.querySelector("[data-slot=toast-provider] [aria-label='Update Now']");
  const index = list.indexOf(target);
  if (!target || index < 1) return { found: false, index, total: list.length, positive };
  const before = list[index - 1];
  before.focus();
  return { found: true, index, total: list.length, positive,
           predecessor: (before.getAttribute('aria-label') || before.textContent || before.tagName).trim().slice(0, 60),
           closeFollows: list.indexOf(document.querySelector("[data-slot=toast-provider] [data-slot=toast-close]")) > index };
})()
